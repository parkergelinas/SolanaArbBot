/**
 * Strategy E: LST Spread Arbitrage
 *
 * Liquid Staking Tokens (mSOL, jitoSOL, bSOL) should trade near their
 * staking-program exchange rate vs SOL.  When Raydium and Orca quote
 * materially different prices for the same LST/SOL pair, we can exploit
 * the venue spread just like cross-DEX arb — but the LST pairs tend to
 * have tighter liquidity and thus wider bps spreads than SOL/USDC.
 *
 * Edge mechanics:
 *   1. Quote LST→SOL on Raydium only.
 *   2. Quote LST→SOL on Orca/Whirlpool only.
 *   3. If |spread| > FEE_FLOOR_BPS, buy LST on the cheaper venue, sell on dearer.
 *
 * Expected spread window: 5–50 bps (wider than SOL/USDC due to lower TVL).
 * Fee floor: ~20 bps round-trip (Orca 5 bps + Orca 5 bps buy+sell,
 *            or Orca 5 bps + Raydium 25 bps = 30 bps worst-case).
 */

import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { StrategyContext, TradeDecision } from './types.js';
import type { ScannableStrategy } from '../signals/types.js';
import { estimateFeesUsd } from './scorer.js';
import {
  RAYDIUM_DEXES,
  ORCA_DEXES,
  scanCrossDexPairs,
  type CrossDexScanPair,
} from './cross-dex-scanner.js';

// ── LST token mint addresses ──────────────────────────────────────────────────
export const MSOL_MINT   = 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So';
export const JITOSOL_MINT = 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn';
export const BSOL_MINT   = 'bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1';
export const SOL_MINT    = 'So11111111111111111111111111111111111111112';

/** Fee floor for LST pairs — Orca 5 bps × 2 legs minimum. */
export const LST_FEE_FLOOR_BPS = 20;

export const LST_SCAN_PAIRS: CrossDexScanPair[] = [
  { label: 'mSOL/SOL',    baseMint: MSOL_MINT,    quoteMint: SOL_MINT, baseDecimals: 9, quoteDecimals: 9 },
  { label: 'jitoSOL/SOL', baseMint: JITOSOL_MINT, quoteMint: SOL_MINT, baseDecimals: 9, quoteDecimals: 9 },
  { label: 'bSOL/SOL',    baseMint: BSOL_MINT,    quoteMint: SOL_MINT, baseDecimals: 9, quoteDecimals: 9 },
];

export interface LstSpreadConfig {
  /** Trade size in LST UI units (e.g. 1.0 = 1 mSOL). */
  tradeSizeLst: number;
  /** Minimum net spread in bps to signal a trade. */
  minNetSpreadBps: number;
  /** Scan concurrency — keep low to avoid rate limits. */
  concurrency: number;
}

export const DEFAULT_LST_CONFIG: LstSpreadConfig = {
  tradeSizeLst: 1.0,
  minNetSpreadBps: 5,
  concurrency: 2,
};

export class LstSpreadArbStrategy implements ScannableStrategy {
  readonly id = 'lst_spread_arb';

  constructor(
    private readonly client: JupiterClient,
    private readonly cfg: LstSpreadConfig = DEFAULT_LST_CONFIG,
  ) {}

  async scan(state: MarketState): Promise<MarketState> {
    const result = await scanCrossDexPairs(
      this.client,
      LST_SCAN_PAIRS,
      this.cfg.tradeSizeLst,
      { slippageBps: 30, concurrency: this.cfg.concurrency },
    );

    // Override fee floor for LST pairs — tighter than Raydium/Orca mixed pairs
    const adjusted = result.opportunities.map((opp) => ({
      ...opp,
      netSpreadBps: opp.spreadBps - LST_FEE_FLOOR_BPS,
      profitable: (opp.spreadBps - LST_FEE_FLOOR_BPS) > 0 &&
                  opp.raydiumPriceOut > 0 && opp.orcaPriceOut > 0,
    }));

    return { ...state, crossDexOpportunities: adjusted };
  }

  evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    const opps = state.crossDexOpportunities ?? [];
    // Filter to only LST pairs and only profitable ones
    const candidates = opps.filter(
      (o) => LST_SCAN_PAIRS.some((p) => p.label === o.pairLabel) && o.profitable,
    );
    if (candidates.length === 0) return null;

    // Pick the widest spread
    const best = candidates[0]!;
    const solPrice = state.solPriceUsd || 150;
    const notionalUsd = this.cfg.tradeSizeLst * (state.pricesUsd[best.baseMint] ?? solPrice);

    const feesUsd = estimateFeesUsd({
      solPriceUsd: solPrice,
      slippageBps: ctx.slippageBps,
      notionalUsd,
    });

    const grossUsd = notionalUsd * (best.spreadBps / 10_000);
    const netProfitUsd = grossUsd - feesUsd.totalUsd;

    const base: TradeDecision = {
      strategyId: this.id,
      pairLabel: best.pairLabel,
      inputMint: best.cheaperDex === 'orca' ? best.baseMint : best.quoteMint,
      outputMint: best.cheaperDex === 'orca' ? best.quoteMint : best.baseMint,
      amountInAtomic: String(Math.round(this.cfg.tradeSizeLst * 1e9)),
      expectedOutAtomic: best.cheaperDex === 'orca'
        ? String(best.orcaQuote?.outAmount ?? '0')
        : String(best.raydiumQuote?.outAmount ?? '0'),
      netProfitUsd,
      grossSpreadBps: best.spreadBps,
      routeQualityScore: 0.85,
      freshnessScore: Math.max(0, 1 - (Date.now() - best.capturedAtMs) / 5000),
      metadata: {
        cheaperDex: best.cheaperDex,
        dearerDex: best.dearerDex,
        spreadBps: best.spreadBps,
        netSpreadBps: best.netSpreadBps,
      },
    };

    if (netProfitUsd < ctx.minProfitUsd) {
      return { ...base, rejectionReason: 'below_min_profit' };
    }

    return base;
  }
}
