/**
 * Paper-mode route divergence simulator.
 *
 * When Jupiter's quote API is rate-limited (429), this generates synthetic
 * RouteDivergenceSnapshots using live price data from DexScreener. The
 * simulated divergence models real-world AMM fee tier differences between
 * routes (e.g. Orca 0.05% vs Raydium 0.25%).
 *
 * PAPER MODE ONLY — never used in live trading.
 */

import type { ScanPair } from './arb-scanner.js';
import type { RouteDivergenceSnapshot } from '../market/state.js';

/**
 * Simulate route divergence for a single pair using current price data.
 *
 * @param pair         The trading pair (baseMint/quoteMint + decimals)
 * @param amountUi     Trade size in base-token UI units (e.g. 0.35 SOL)
 * @param priceUsd     USD price of baseMint
 * @param quotePriceUsd USD price of quoteMint (use 1.0 for USDC)
 * @param scanTick     Monotonic scan counter — seeds the randomised divergence
 */
export function simulatePaperDivergence(
  pair: ScanPair,
  amountUi: number,
  priceUsd: number,
  quotePriceUsd: number,
  scanTick: number,
): RouteDivergenceSnapshot | null {
  if (priceUsd <= 0 || quotePriceUsd <= 0) return null;

  const now = Date.now();
  const amountInAtomic = Math.round(amountUi * 10 ** pair.baseDecimals);

  // Price of base in quote-token units (e.g. SOL price in USDC)
  const baseInQuote = priceUsd / quotePriceUsd;

  // ── Simulate two AMM routing policies ────────────────────────────────────
  // "restricted" = single-hop with higher fee tier (Raydium v4: 0.25%)
  // "unrestricted" = multi-hop, finds cheaper route (Orca 0.05% pool)
  const feeRestricted = 0.0025; // 0.25%
  const feeUnrestricted = 0.0005; // 0.05%

  // Seed a small stochastic noise so divergence varies realistically per pair/tick
  const seed = Math.abs(Math.sin(scanTick * 1_000_003 + pair.baseMint.charCodeAt(2) * 997));
  // noise: -0.5..+0.5 bps spread from the fee-tier gap
  const noiseFraction = (seed - 0.5) * 0.0001;

  const effRestricted = feeRestricted * 2 + noiseFraction;   // forward + reverse
  const effUnrestricted = feeUnrestricted * 2 - noiseFraction; // forward + reverse

  // ── Forward leg (base → quote) ───────────────────────────────────────────
  const fwdOutRestricted = amountUi * baseInQuote * (1 - feeRestricted);
  const fwdOutUnrestricted = amountUi * baseInQuote * (1 - feeUnrestricted);

  const fwdOutRestAtomic = Math.round(fwdOutRestricted * 10 ** pair.quoteDecimals);
  const fwdOutUnrestAtomic = Math.round(fwdOutUnrestricted * 10 ** pair.quoteDecimals);

  // ── Reverse leg (quote → base) ───────────────────────────────────────────
  const revOutRestricted = fwdOutRestricted / baseInQuote * (1 - feeRestricted);
  const revOutUnrestricted = fwdOutUnrestricted / baseInQuote * (1 - feeUnrestricted);

  const revOutRestAtomic = Math.round(revOutRestricted * 10 ** pair.baseDecimals);
  const revOutUnrestAtomic = Math.round(revOutUnrestricted * 10 ** pair.baseDecimals);

  // ── Divergence ────────────────────────────────────────────────────────────
  const mid = (revOutUnrestricted + revOutRestricted) / 2;
  const divergenceBps = mid > 0
    ? Math.round(((revOutUnrestricted - revOutRestricted) / mid) * 10_000)
    : 0;

  const constructions = [
    {
      label: 'restricted',
      restrictIntermediateTokens: true,
      forward: {
        request: {
          inputMint: pair.baseMint,
          outputMint: pair.quoteMint,
          amount: amountInAtomic,
          slippageBps: 50,
          restrictIntermediateTokens: true,
        },
        response: {
          inAmount: String(amountInAtomic),
          outAmount: String(fwdOutRestAtomic),
          priceImpactPct: (feeRestricted * 100).toFixed(3),
          routePlan: [{ percent: 100 }],
        },
        capturedAtMs: now,
      },
      reverse: {
        request: {
          inputMint: pair.quoteMint,
          outputMint: pair.baseMint,
          amount: fwdOutRestAtomic,
          slippageBps: 50,
          restrictIntermediateTokens: true,
        },
        response: {
          inAmount: String(fwdOutRestAtomic),
          outAmount: String(revOutRestAtomic),
          priceImpactPct: (feeRestricted * 100).toFixed(3),
          routePlan: [{ percent: 100 }],
        },
        capturedAtMs: now,
      },
    },
    {
      label: 'unrestricted',
      restrictIntermediateTokens: false,
      forward: {
        request: {
          inputMint: pair.baseMint,
          outputMint: pair.quoteMint,
          amount: amountInAtomic,
          slippageBps: 50,
          restrictIntermediateTokens: false,
        },
        response: {
          inAmount: String(amountInAtomic),
          outAmount: String(fwdOutUnrestAtomic),
          priceImpactPct: (feeUnrestricted * 100).toFixed(3),
          routePlan: [{ percent: 100 }],
        },
        capturedAtMs: now,
      },
      reverse: {
        request: {
          inputMint: pair.quoteMint,
          outputMint: pair.baseMint,
          amount: fwdOutUnrestAtomic,
          slippageBps: 50,
          restrictIntermediateTokens: false,
        },
        response: {
          inAmount: String(fwdOutUnrestAtomic),
          outAmount: String(revOutUnrestAtomic),
          priceImpactPct: (feeUnrestricted * 100).toFixed(3),
          routePlan: [{ percent: 100 }],
        },
        capturedAtMs: now,
      },
    },
  ];

  return {
    pairLabel: pair.label,
    constructions,
    bestConstructionLabel: 'unrestricted',
    divergenceBps,
  };
}
