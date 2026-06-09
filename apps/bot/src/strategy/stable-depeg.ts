/**
 * Strategy G: Stablecoin Micro-Depeg Arbitrage
 *
 * USDC, USDT, PYUSD, and USDS should all trade at exactly $1.00.
 * When liquidity imbalances push one stablecoin's DEX price off peg
 * vs another, we can profitably swap between them.
 *
 * Pairs monitored:
 *   USDC → USDT → USDC  (round-trip peg deviation check)
 *   USDC → PYUSD → USDC
 *   USDT → USDC → USDT
 *
 * Edge: 3–15 bps during high-volume periods (USDC/USDT is the deepest
 *        stablecoin pair on Solana with > $500M daily volume).
 *
 * Fee floor: ~6 bps (Orca CLMM stablecoin pool is typically 1 bp fee;
 *            worst-case Raydium stable = 5 bps × 2 legs = 10 bps).
 * Minimum trade size: $500+ to overcome fixed priority fee ($0.01).
 */

import type { JupiterClient } from '../jupiter/client.js';
import { uiToAtomic, atomicToUi } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { ScannableStrategy } from '../signals/types.js';
import type { StrategyContext, TradeDecision } from './types.js';
import { estimateFeesUsd } from './scorer.js';

// ── Stablecoin mints ──────────────────────────────────────────────────────────
export const USDC_MINT  = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
export const USDT_MINT  = 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB';
export const PYUSD_MINT = '2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo';
export const USDS_MINT  = 'USDSwr9ApdHk5bvJKMjzff41FfuX8bSxdKcR81vTwcA';

export interface StableDepegPair {
  label: string;
  mintIn: string;
  mintOut: string;
  decimalsIn: number;
  decimalsOut: number;
}

export const STABLE_PAIRS: StableDepegPair[] = [
  { label: 'USDC/USDT', mintIn: USDC_MINT, mintOut: USDT_MINT, decimalsIn: 6, decimalsOut: 6 },
  { label: 'USDT/USDC', mintIn: USDT_MINT, mintOut: USDC_MINT, decimalsIn: 6, decimalsOut: 6 },
  { label: 'USDC/PYUSD', mintIn: USDC_MINT, mintOut: PYUSD_MINT, decimalsIn: 6, decimalsOut: 6 },
];

export interface DepegQuote {
  pair: StableDepegPair;
  /** Input amount in UI stablecoins (e.g. 500 USDC). */
  inputUi: number;
  outputUi: number;
  /** Deviation from 1:1 peg in bps — positive = profitable (more out than in). */
  pegDeviationBps: number;
  profitable: boolean;
  capturedAtMs: number;
  error?: string;
}

const QUOTE_TIMEOUT_MS = 6_000;

function withTimeout<T>(p: Promise<T>, ms: number): Promise<T> {
  return Promise.race([
    p,
    new Promise<never>((_, rej) => setTimeout(() => rej(new Error('timeout')), ms)),
  ]);
}

async function quoteDepeg(
  client: JupiterClient,
  pair: StableDepegPair,
  amountUi: number,
  slippageBps: number,
): Promise<DepegQuote> {
  const now = Date.now();
  try {
    const amountAtomic = uiToAtomic(amountUi, pair.decimalsIn);
    const quote = await withTimeout(
      client.getQuote({
        inputMint: pair.mintIn,
        outputMint: pair.mintOut,
        amount: amountAtomic.toString(),
        slippageBps,
      }),
      QUOTE_TIMEOUT_MS,
    );

    const outputUi = atomicToUi(quote.outAmount, pair.decimalsOut);
    // 1 USDC should give exactly 1 USDT — deviation from parity
    const pegDeviationBps = Math.round((outputUi / amountUi - 1) * 10_000);

    return {
      pair,
      inputUi: amountUi,
      outputUi,
      pegDeviationBps,
      profitable: pegDeviationBps > 0,
      capturedAtMs: now,
    };
  } catch (err) {
    return {
      pair,
      inputUi: amountUi,
      outputUi: 0,
      pegDeviationBps: -10_000,
      profitable: false,
      capturedAtMs: now,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export interface StableDepegConfig {
  /** Trade size in stablecoins (USD). Higher = better fee amortisation. */
  tradeAmountUsd: number;
  /** Minimum peg deviation in bps to signal a trade. */
  minDeviationBps: number;
  concurrency: number;
}

export const DEFAULT_STABLE_CONFIG: StableDepegConfig = {
  tradeAmountUsd: 500,
  minDeviationBps: 3,
  concurrency: 3,
};

const STABLE_KEY = '__stable_depeg_results';

export class StableDepegStrategy implements ScannableStrategy {
  readonly id = 'stable_depeg_arb';

  constructor(
    private readonly client: JupiterClient,
    private readonly cfg: StableDepegConfig = DEFAULT_STABLE_CONFIG,
  ) {}

  async scan(state: MarketState): Promise<MarketState> {
    const results: DepegQuote[] = [];

    for (let i = 0; i < STABLE_PAIRS.length; i += this.cfg.concurrency) {
      const batch = STABLE_PAIRS.slice(i, i + this.cfg.concurrency);
      const batchResults = await Promise.all(
        batch.map((pair) => quoteDepeg(this.client, pair, this.cfg.tradeAmountUsd, 20)),
      );
      results.push(...batchResults);
    }

    // Sort by deviation descending
    results.sort((a, b) => b.pegDeviationBps - a.pegDeviationBps);

    return { ...state, [STABLE_KEY]: results } as MarketState & Record<string, unknown>;
  }

  evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    const results: DepegQuote[] = (state as unknown as Record<string, unknown>)[STABLE_KEY] as DepegQuote[] ?? [];
    if (results.length === 0) return null;

    const solPrice = state.solPriceUsd || 150;

    for (const r of results) {
      if (r.error || !r.profitable || r.pegDeviationBps < this.cfg.minDeviationBps) continue;

      const notionalUsd = this.cfg.tradeAmountUsd;
      const feesUsd = estimateFeesUsd({
        solPriceUsd: solPrice,
        slippageBps: ctx.slippageBps,
        notionalUsd,
        priorityFeeLamports: 20_000, // lower priority for stable swaps — not time-critical
      });

      const grossUsd = notionalUsd * (r.pegDeviationBps / 10_000);
      const netProfitUsd = grossUsd - feesUsd.totalUsd;

      const base: TradeDecision = {
        strategyId: this.id,
        pairLabel: r.pair.label,
        inputMint: r.pair.mintIn,
        outputMint: r.pair.mintOut,
        amountInAtomic: String(uiToAtomic(r.inputUi, r.pair.decimalsIn)),
        expectedOutAtomic: String(uiToAtomic(r.outputUi, r.pair.decimalsOut)),
        netProfitUsd,
        grossSpreadBps: r.pegDeviationBps,
        routeQualityScore: 0.95,
        freshnessScore: Math.max(0, 1 - (Date.now() - r.capturedAtMs) / 3000),
        metadata: { pegDeviationBps: r.pegDeviationBps, outputUi: r.outputUi },
      };

      if (netProfitUsd < ctx.minProfitUsd) {
        return { ...base, rejectionReason: 'below_min_profit' };
      }

      return base;
    }

    return null;
  }
}
