/**
 * Strategy F: 3-Leg Triangular Arbitrage
 *
 * Exploits price inefficiencies across three token pairs in a closed cycle:
 *   SOL → TOKEN_A → TOKEN_B → SOL
 *
 * Example cycles:
 *   SOL → USDC → BONK → SOL   (detects BONK mispricing vs USDC vs SOL)
 *   SOL → USDC → WIF  → SOL
 *   SOL → USDC → JTO  → SOL
 *   SOL → JTO  → USDC → SOL   (reverse cycle)
 *
 * Profit condition:
 *   (out_B_amount / in_SOL_amount) - 1 > fee_total
 *
 * Fees: 3 Jupiter swaps × ~25 bps average = ~75 bps worst-case.
 * Expected profitable threshold: > 80 bps gross spread.
 *
 * Implementation:
 *   We get Jupiter quotes for each individual leg, chain the amounts,
 *   and compute the net return.  Jupiter's smart routing will pick the
 *   best sub-path for each leg.
 */

import type { JupiterClient } from '../jupiter/client.js';
import { uiToAtomic, atomicToUi } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { ScannableStrategy } from '../signals/types.js';
import type { StrategyContext, TradeDecision } from './types.js';
import { estimateFeesUsd } from './scorer.js';

// ── Well-known mint addresses ─────────────────────────────────────────────────
const SOL_MINT  = 'So11111111111111111111111111111111111111112';
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
const BONK_MINT = 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263';
const WIF_MINT  = 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm';
const JTO_MINT  = 'jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL';
const PYTH_MINT = 'HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3';

const QUOTE_TIMEOUT_MS = 8_000;

function withTimeout<T>(p: Promise<T>, ms: number): Promise<T> {
  return Promise.race([
    p,
    new Promise<never>((_, rej) => setTimeout(() => rej(new Error('timeout')), ms)),
  ]);
}

export interface TriangleCycle {
  label: string;
  /** Mint addresses for the 3-leg cycle [start, pivot1, pivot2]. */
  mints: [string, string, string];
  /** Decimal counts for each mint. */
  decimals: [number, number, number];
}

/** Canonical cycle definitions — start and end at SOL. */
export const TRIANGLE_CYCLES: TriangleCycle[] = [
  { label: 'SOL→USDC→BONK→SOL', mints: [SOL_MINT, USDC_MINT, BONK_MINT], decimals: [9, 6, 5] },
  { label: 'SOL→USDC→WIF→SOL',  mints: [SOL_MINT, USDC_MINT, WIF_MINT],  decimals: [9, 6, 6] },
  { label: 'SOL→USDC→JTO→SOL',  mints: [SOL_MINT, USDC_MINT, JTO_MINT],  decimals: [9, 6, 9] },
  { label: 'SOL→USDC→PYTH→SOL', mints: [SOL_MINT, USDC_MINT, PYTH_MINT], decimals: [9, 6, 6] },
  { label: 'SOL→BONK→USDC→SOL', mints: [SOL_MINT, BONK_MINT, USDC_MINT], decimals: [9, 5, 6] },
  { label: 'SOL→JTO→USDC→SOL',  mints: [SOL_MINT, JTO_MINT, USDC_MINT],  decimals: [9, 9, 6] },
];

export interface TriangleResult {
  cycle: TriangleCycle;
  inputAmountUi: number;
  leg1OutAtomic: string;
  leg2OutAtomic: string;
  finalOutAtomic: string;
  finalOutUi: number;
  grossReturnBps: number;
  profitable: boolean;
  capturedAtMs: number;
  error?: string;
}

/** Quote one triangular cycle — returns null if any leg fails. */
async function quoteTriangle(
  client: JupiterClient,
  cycle: TriangleCycle,
  inputAmountSolUi: number,
  slippageBps: number,
): Promise<TriangleResult> {
  const now = Date.now();
  const [m0, m1, m2] = cycle.mints;
  const [d0, d1, d2] = cycle.decimals;
  const inputAtomic = uiToAtomic(inputAmountSolUi, d0);

  try {
    // Leg 1: SOL → pivot1
    const q1 = await withTimeout(
      client.getQuote({ inputMint: m0, outputMint: m1, amount: inputAtomic.toString(), slippageBps }),
      QUOTE_TIMEOUT_MS,
    );

    // Leg 2: pivot1 → pivot2
    const q2 = await withTimeout(
      client.getQuote({ inputMint: m1, outputMint: m2, amount: q1.outAmount, slippageBps }),
      QUOTE_TIMEOUT_MS,
    );

    // Leg 3: pivot2 → SOL
    const q3 = await withTimeout(
      client.getQuote({ inputMint: m2, outputMint: m0, amount: q2.outAmount, slippageBps }),
      QUOTE_TIMEOUT_MS,
    );

    const finalOutUi = atomicToUi(q3.outAmount, d0);
    const grossReturnBps = Math.round((finalOutUi / inputAmountSolUi - 1) * 10_000);

    return {
      cycle,
      inputAmountUi: inputAmountSolUi,
      leg1OutAtomic: q1.outAmount,
      leg2OutAtomic: q2.outAmount,
      finalOutAtomic: q3.outAmount,
      finalOutUi,
      grossReturnBps,
      profitable: grossReturnBps > 0,
      capturedAtMs: now,
    };
  } catch (err) {
    return {
      cycle,
      inputAmountUi: inputAmountSolUi,
      leg1OutAtomic: '0',
      leg2OutAtomic: '0',
      finalOutAtomic: '0',
      finalOutUi: 0,
      grossReturnBps: -10000,
      profitable: false,
      capturedAtMs: now,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export interface TriangleArbConfig {
  /** Trade size in SOL. */
  tradeSizeSol: number;
  /** Minimum gross return in bps to consider profitable before fees. */
  minGrossBps: number;
  /** Max concurrent cycle quotes. */
  concurrency: number;
}

export const DEFAULT_TRIANGLE_CONFIG: TriangleArbConfig = {
  tradeSizeSol: 0.5,
  minGrossBps: 30,
  concurrency: 2,
};

/** Triangular arbitrage state stored in MarketState.metadata. */
const TRIANGLE_KEY = '__triangle_results';

export class TriangleArbStrategy implements ScannableStrategy {
  readonly id = 'triangle_arb';

  constructor(
    private readonly client: JupiterClient,
    private readonly cfg: TriangleArbConfig = DEFAULT_TRIANGLE_CONFIG,
  ) {}

  async scan(state: MarketState): Promise<MarketState> {
    const results: TriangleResult[] = [];

    // Process cycles in batches limited by concurrency
    for (let i = 0; i < TRIANGLE_CYCLES.length; i += this.cfg.concurrency) {
      const batch = TRIANGLE_CYCLES.slice(i, i + this.cfg.concurrency);
      const batchResults = await Promise.all(
        batch.map((cycle) =>
          quoteTriangle(this.client, cycle, this.cfg.tradeSizeSol, 50),
        ),
      );
      results.push(...batchResults);
    }

    // Sort by grossReturnBps descending
    results.sort((a, b) => b.grossReturnBps - a.grossReturnBps);

    return {
      ...state,
      // Store in pumpEdges as a reuse of the metadata slot (triangle results JSON)
      // Using metadata field on state — store as a well-known key
      [TRIANGLE_KEY]: results,
    } as MarketState & Record<string, unknown>;
  }

  evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    const results: TriangleResult[] = (state as unknown as Record<string, unknown>)[TRIANGLE_KEY] as TriangleResult[] ?? [];
    if (results.length === 0) return null;

    const solPrice = state.solPriceUsd || 150;
    const notionalUsd = this.cfg.tradeSizeSol * solPrice;

    // 3-leg fees: 3 × priority_fee + 3 × slippage
    const feesUsd = estimateFeesUsd({
      solPriceUsd: solPrice,
      slippageBps: ctx.slippageBps * 3,
      notionalUsd,
      priorityFeeLamports: 150_000,  // 3 × 50k lamports
    });

    for (const r of results) {
      if (r.error || r.grossReturnBps < this.cfg.minGrossBps) continue;

      const grossUsd = notionalUsd * (r.grossReturnBps / 10_000);
      const netProfitUsd = grossUsd - feesUsd.totalUsd;

      const base: TradeDecision = {
        strategyId: this.id,
        pairLabel: r.cycle.label,
        inputMint: r.cycle.mints[0],
        outputMint: r.cycle.mints[0],
        amountInAtomic: String(uiToAtomic(this.cfg.tradeSizeSol, r.cycle.decimals[0])),
        expectedOutAtomic: r.finalOutAtomic,
        netProfitUsd,
        grossSpreadBps: r.grossReturnBps,
        routeQualityScore: 0.80,
        freshnessScore: Math.max(0, 1 - (Date.now() - r.capturedAtMs) / 5000),
        metadata: {
          cycle: r.cycle.label,
          grossReturnBps: r.grossReturnBps,
          finalOutUi: r.finalOutUi,
          inputAmountUi: r.inputAmountUi,
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
