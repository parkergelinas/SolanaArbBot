/**
 * Strategy I: Route Diversity Sweep
 *
 * Jupiter supports routing through many DEXes beyond just Raydium and Orca.
 * This strategy systematically tests 6+ DEX combinations per pair to find
 * the widest spread between any two route constructions.
 *
 * DEX groups tested:
 *   1. Raydium (standard + CLMM + CP)
 *   2. Orca / Whirlpool
 *   3. Phoenix (CLOB — central limit order book)
 *   4. Lifinity (proactive market maker)
 *   5. Meteora (dynamic vault AMM)
 *   6. Jupiter unrestricted (best aggregate route)
 *
 * For each DEX group, we get output for the same input amount.
 * The widest gap between the worst and best DEX route = the arb edge.
 *
 * Trade: submit via Jupiter unrestricted (best route) to capture full edge.
 * Risk: DEXes with low liquidity may show phantom spreads that close on
 *       execution (slippage).  We apply a 40% haircut to raw spread.
 *
 * Expected spreads: 5–200 bps depending on pair liquidity and DEX depth.
 * Fee floor: ~25 bps (Raydium 25 bps + priority fee).
 */

import type { JupiterClient } from '../jupiter/client.js';
import { uiToAtomic, atomicToUi } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { ScannableStrategy } from '../signals/types.js';
import type { StrategyContext, TradeDecision } from './types.js';
import { estimateFeesUsd } from './scorer.js';

// ── DEX group definitions ─────────────────────────────────────────────────────
export interface DexGroup {
  label: string;
  dexes: string;
}

export const DEX_GROUPS: DexGroup[] = [
  { label: 'raydium',  dexes: 'Raydium,Raydium CLMM,Raydium CP' },
  { label: 'orca',     dexes: 'Orca,Whirlpool' },
  { label: 'phoenix',  dexes: 'Phoenix' },
  { label: 'lifinity', dexes: 'Lifinity V2' },
  { label: 'meteora',  dexes: 'Meteora,Meteora DLMM' },
  { label: 'unrestricted', dexes: '' },  // empty = Jupiter best route
];

/** Haircut applied to raw spread to model liquidity risk. */
const SPREAD_HAIRCUT = 0.40;

const SOL_MINT  = 'So11111111111111111111111111111111111111112';
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
const WIF_MINT  = 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm';
const JTO_MINT  = 'jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL';
const MSOL_MINT = 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So';

export interface DiversityScanPair {
  label: string;
  baseMint: string;
  quoteMint: string;
  baseDecimals: number;
  quoteDecimals: number;
  tradeSizeUi: number;
}

export const DIVERSITY_PAIRS: DiversityScanPair[] = [
  { label: 'SOL/USDC',  baseMint: SOL_MINT,  quoteMint: USDC_MINT, baseDecimals: 9, quoteDecimals: 6, tradeSizeUi: 0.5 },
  { label: 'WIF/USDC',  baseMint: WIF_MINT,  quoteMint: USDC_MINT, baseDecimals: 6, quoteDecimals: 6, tradeSizeUi: 10  },
  { label: 'JTO/USDC',  baseMint: JTO_MINT,  quoteMint: USDC_MINT, baseDecimals: 9, quoteDecimals: 6, tradeSizeUi: 1   },
  { label: 'mSOL/SOL',  baseMint: MSOL_MINT, quoteMint: SOL_MINT,  baseDecimals: 9, quoteDecimals: 9, tradeSizeUi: 1   },
];

interface DexQuoteResult {
  dexLabel: string;
  priceOut: number;  // quote tokens per base token
  outAtomic: string;
  success: boolean;
}

export interface DiversityOpportunity {
  pairLabel: string;
  baseMint: string;
  quoteMint: string;
  baseDecimals: number;
  tradeSizeUi: number;
  /** DEX with highest output (sell here). */
  bestDex: DexQuoteResult;
  /** DEX with lowest output (buy here in the opposite direction, or just note it). */
  worstDex: DexQuoteResult;
  /** Spread in bps between best and worst routes (before haircut). */
  rawSpreadBps: number;
  /** Spread after liquidity haircut. */
  adjustedSpreadBps: number;
  dexResults: DexQuoteResult[];
  capturedAtMs: number;
}

const QUOTE_TIMEOUT_MS = 7_000;

function withTimeout<T>(p: Promise<T>, ms: number): Promise<T> {
  return Promise.race([
    p,
    new Promise<never>((_, rej) => setTimeout(() => rej(new Error('timeout')), ms)),
  ]);
}

async function fetchGroupQuote(
  client: JupiterClient,
  pair: DiversityScanPair,
  dexGroup: DexGroup,
  amountAtomic: string,
  slippageBps: number,
): Promise<DexQuoteResult> {
  try {
    const opts: Parameters<typeof client.getQuote>[0] = {
      inputMint: pair.baseMint,
      outputMint: pair.quoteMint,
      amount: amountAtomic,
      slippageBps,
    };
    if (dexGroup.dexes) {
      opts.dexes = dexGroup.dexes;
    }
    const q = await withTimeout(client.getQuote(opts), QUOTE_TIMEOUT_MS);
    const inAtomic = Number(q.inAmount);
    const outAtomic = Number(q.outAmount);
    if (!inAtomic || !outAtomic) throw new Error('zero amounts');
    const priceOut = (outAtomic / 10 ** pair.quoteDecimals) / (inAtomic / 10 ** pair.baseDecimals);
    return { dexLabel: dexGroup.label, priceOut, outAtomic: q.outAmount, success: true };
  } catch {
    return { dexLabel: dexGroup.label, priceOut: 0, outAtomic: '0', success: false };
  }
}

async function scanPairDiversity(
  client: JupiterClient,
  pair: DiversityScanPair,
  slippageBps: number,
): Promise<DiversityOpportunity> {
  const amountAtomic = uiToAtomic(pair.tradeSizeUi, pair.baseDecimals).toString();
  const now = Date.now();

  // Query all DEX groups concurrently
  const results = await Promise.all(
    DEX_GROUPS.map((dg) => fetchGroupQuote(client, pair, dg, amountAtomic, slippageBps)),
  );

  const valid = results.filter((r) => r.success && r.priceOut > 0);

  let bestDex = valid[0] ?? results[0]!;
  let worstDex = valid[0] ?? results[0]!;
  for (const r of valid) {
    if (r.priceOut > bestDex.priceOut) bestDex = r;
    if (r.priceOut < worstDex.priceOut) worstDex = r;
  }

  const rawSpreadBps = bestDex.priceOut > 0 && worstDex.priceOut > 0
    ? Math.round((bestDex.priceOut / worstDex.priceOut - 1) * 10_000)
    : 0;
  const adjustedSpreadBps = Math.round(rawSpreadBps * (1 - SPREAD_HAIRCUT));

  return {
    pairLabel: pair.label,
    baseMint: pair.baseMint,
    quoteMint: pair.quoteMint,
    baseDecimals: pair.baseDecimals,
    tradeSizeUi: pair.tradeSizeUi,
    bestDex,
    worstDex,
    rawSpreadBps,
    adjustedSpreadBps,
    dexResults: results,
    capturedAtMs: now,
  };
}

export interface RouteDiversityConfig {
  minAdjustedSpreadBps: number;
  concurrency: number;
}

export const DEFAULT_DIVERSITY_CONFIG: RouteDiversityConfig = {
  minAdjustedSpreadBps: 15,
  concurrency: 2,
};

const DIVERSITY_KEY = '__diversity_opportunities';

export class RouteDiversitySweepStrategy implements ScannableStrategy {
  readonly id = 'route_diversity_sweep';

  constructor(
    private readonly client: JupiterClient,
    private readonly cfg: RouteDiversityConfig = DEFAULT_DIVERSITY_CONFIG,
  ) {}

  async scan(state: MarketState): Promise<MarketState> {
    const opps: DiversityOpportunity[] = [];

    for (let i = 0; i < DIVERSITY_PAIRS.length; i += this.cfg.concurrency) {
      const batch = DIVERSITY_PAIRS.slice(i, i + this.cfg.concurrency);
      const results = await Promise.all(
        batch.map((pair) => scanPairDiversity(this.client, pair, 50)),
      );
      opps.push(...results);
    }

    opps.sort((a, b) => b.adjustedSpreadBps - a.adjustedSpreadBps);

    return { ...state, [DIVERSITY_KEY]: opps } as MarketState & Record<string, unknown>;
  }

  evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    const opps: DiversityOpportunity[] = (state as unknown as Record<string, unknown>)[DIVERSITY_KEY] as DiversityOpportunity[] ?? [];
    if (opps.length === 0) return null;

    const solPrice = state.solPriceUsd || 150;

    for (const opp of opps) {
      if (opp.adjustedSpreadBps < this.cfg.minAdjustedSpreadBps) continue;
      if (!opp.bestDex.success || !opp.worstDex.success) continue;

      const tokenPrice = state.pricesUsd[opp.baseMint] ?? solPrice;
      const notionalUsd = opp.tradeSizeUi * tokenPrice;

      const feesUsd = estimateFeesUsd({
        solPriceUsd: solPrice,
        slippageBps: ctx.slippageBps,
        notionalUsd,
      });

      const grossUsd = notionalUsd * (opp.adjustedSpreadBps / 10_000);
      const netProfitUsd = grossUsd - feesUsd.totalUsd;

      const base: TradeDecision = {
        strategyId: this.id,
        pairLabel: opp.pairLabel,
        inputMint: opp.baseMint,
        outputMint: opp.quoteMint,
        amountInAtomic: String(uiToAtomic(opp.tradeSizeUi, opp.baseDecimals)),
        expectedOutAtomic: opp.bestDex.outAtomic,
        netProfitUsd,
        grossSpreadBps: opp.rawSpreadBps,
        routeQualityScore: 0.80,
        freshnessScore: Math.max(0, 1 - (Date.now() - opp.capturedAtMs) / 5000),
        metadata: {
          bestDex: opp.bestDex.dexLabel,
          worstDex: opp.worstDex.dexLabel,
          rawSpreadBps: opp.rawSpreadBps,
          adjustedSpreadBps: opp.adjustedSpreadBps,
          dexCount: opp.dexResults.filter((r) => r.success).length,
        },
      };

      if (netProfitUsd < ctx.minProfitUsd) {
        return { ...base, rejectionReason: 'below_min_profit' };
      }

      return base;
    }

    return null;
  }
}
