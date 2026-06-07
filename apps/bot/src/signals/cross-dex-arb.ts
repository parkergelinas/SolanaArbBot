/**
 * Cross-DEX arb strategy — compares Raydium vs Orca real-time prices for the
 * same token pair and signals when the spread exceeds combined fee cost.
 *
 * Two pair sources:
 *   1. Static core pairs (SOL/USDC, JUP/USDC, mSOL/SOL, etc.) — always scanned
 *   2. Pump.fun memecoin registry (PumpTokenRegistry) — rotated in when enabled;
 *      these have wider spreads (50–500 bps) but higher risk
 *
 * Execution model:
 *   Paper  — simulate both legs, record spread in journal for analysis
 *   Live   — Jito bundle: tx1 = buy on cheaperDex, tx2 = sell on dearerDex
 *            (two-transaction atomic bundle; execution gap risk eliminated)
 *
 * Spread thresholds:
 *   Core pairs:   BOT_CROSS_DEX_SPREAD_BPS (default 35 bps — above fee floor)
 *   Pump tokens:  BOT_PUMP_SPREAD_BPS (default 80 bps — wider, riskier)
 */

import type { JupiterClient } from '../jupiter/client.js';
import { uiToAtomic, atomicToUi } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { ScannableStrategy } from './types.js';
import type { StrategyContext, TradeDecision } from '../strategy/types.js';
import {
  scanCrossDexPairs,
  type CrossDexQuote,
  type CrossDexScanPair,
} from '../strategy/cross-dex-scanner.js';
import type { PumpTokenRegistry } from '../market/pump-token-registry.js';
import { SOL_MINT, USDC_MINT } from '../config/env.js';

// ── Default core pairs (native Solana, dual-venue liquidity) ──────────────────
// These are the pairs most likely to have measurable Raydium↔Orca spread.
export const CROSS_DEX_CORE_PAIRS: CrossDexScanPair[] = [
  { label: 'SOL/USDC',    baseMint: SOL_MINT,                                              quoteMint: USDC_MINT,                                             baseDecimals: 9, quoteDecimals: 6 },
  { label: 'JUP/USDC',    baseMint: 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN',        quoteMint: USDC_MINT,                                             baseDecimals: 6, quoteDecimals: 6 },
  { label: 'BONK/USDC',   baseMint: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263',       quoteMint: USDC_MINT,                                             baseDecimals: 5, quoteDecimals: 6 },
  { label: 'WIF/USDC',    baseMint: 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm',       quoteMint: USDC_MINT,                                             baseDecimals: 6, quoteDecimals: 6 },
  { label: 'RAY/USDC',    baseMint: '4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R',       quoteMint: USDC_MINT,                                             baseDecimals: 6, quoteDecimals: 6 },
  { label: 'ORCA/USDC',   baseMint: 'orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE',        quoteMint: USDC_MINT,                                             baseDecimals: 6, quoteDecimals: 6 },
  { label: 'mSOL/SOL',    baseMint: 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So',       quoteMint: SOL_MINT,                                              baseDecimals: 9, quoteDecimals: 9 },
  { label: 'jitoSOL/SOL', baseMint: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn',       quoteMint: SOL_MINT,                                              baseDecimals: 9, quoteDecimals: 9 },
  { label: 'DRIFT/USDC',  baseMint: 'DriFtupJYLTosbwoN8koMbEYSx54aFAVLddW1yMmeKht',       quoteMint: USDC_MINT,                                             baseDecimals: 6, quoteDecimals: 6 },
  { label: 'PYTH/USDC',   baseMint: 'HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3',       quoteMint: USDC_MINT,                                             baseDecimals: 6, quoteDecimals: 6 },
];

export interface CrossDexArbConfig {
  /** Min spread bps for core pairs to signal a trade. */
  coreSpreadThresholdBps: number;
  /** Min spread bps for pump/memecoin pairs to signal a trade. */
  pumpSpreadThresholdBps: number;
  /** Max pairs to scan per tick (controls API rate usage). */
  pairsPerScan: number;
  /** Concurrency for parallel quote fetching. */
  scanConcurrency: number;
  /** Whether to scan pump/memecoin pairs from PumpTokenRegistry. */
  enablePumpPairs: boolean;
}

export const DEFAULT_CROSS_DEX_CONFIG: CrossDexArbConfig = {
  coreSpreadThresholdBps: Number(process.env.BOT_CROSS_DEX_SPREAD_BPS ?? '35'),
  pumpSpreadThresholdBps: Number(process.env.BOT_PUMP_SPREAD_BPS ?? '80'),
  pairsPerScan: Number(process.env.BOT_PAIRS_PER_SCAN ?? '8'),
  scanConcurrency: 3,
  enablePumpPairs: (process.env.BOT_ENABLE_PUMP_SPREADS ?? '1') === '1',
};

export class CrossDexArbStrategy implements ScannableStrategy {
  readonly id = 'cross_dex_arb';
  private scanTick = 0;
  private corePairIndex = 0;
  private lastOpportunities: CrossDexQuote[] = [];

  constructor(
    private readonly client: JupiterClient,
    private readonly tradeAmountUi: number,
    private readonly cfg: CrossDexArbConfig = DEFAULT_CROSS_DEX_CONFIG,
    private readonly pumpRegistry: PumpTokenRegistry | null = null,
  ) {}

  async scan(state: MarketState): Promise<MarketState> {
    this.scanTick += 1;

    // Build pair batch: rotate through core pairs + prepend any hot pump pairs
    const coreBatch = this.nextCoreBatch();
    const pumpBatch = this.cfg.enablePumpPairs && this.pumpRegistry
      ? this.pumpRegistry.pairs.slice(0, Math.max(0, this.cfg.pairsPerScan - coreBatch.length))
      : [];

    const batch = [...coreBatch, ...pumpBatch];
    if (batch.length === 0) return state;

    try {
      const result = await scanCrossDexPairs(
        this.client,
        batch,
        this.tradeAmountUi,
        { slippageBps: 50, concurrency: this.cfg.scanConcurrency },
      );
      this.lastOpportunities = result.opportunities;
    } catch {
      // On rate-limit or network error, keep last opportunities
    }

    return { ...state, crossDexOpportunities: this.lastOpportunities };
  }

  evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    const opps = state.crossDexOpportunities ?? this.lastOpportunities;
    if (!opps || opps.length === 0) return null;

    let best: TradeDecision | null = null;
    let bestProfit = -Infinity;

    for (const opp of opps) {
      const decision = this.evaluateOne(opp, state, ctx);
      if (decision && decision.netProfitUsd > bestProfit) {
        bestProfit = decision.netProfitUsd;
        best = decision;
      }
    }

    return best;
  }

  private evaluateOne(
    opp: CrossDexQuote,
    state: MarketState,
    ctx: StrategyContext,
  ): TradeDecision | null {
    if (opp.spreadBps === 0) return null;

    // Classify as pump or core pair
    const isPumpPair = !CROSS_DEX_CORE_PAIRS.some((p) => p.label === opp.pairLabel);
    const spreadThreshold = isPumpPair
      ? this.cfg.pumpSpreadThresholdBps
      : this.cfg.coreSpreadThresholdBps;

    // Estimated profit: netSpreadBps × notional
    const basePrice = state.pricesUsd[opp.baseMint] ?? 0;
    const notionalUsd = this.tradeAmountUi * basePrice;
    const grossProfitUsd = (opp.netSpreadBps / 10_000) * notionalUsd;

    // Estimated transaction costs: two Jupiter swaps + priority fees
    // Rough: 2 × 0.000005 SOL base + priority fees ≈ $0.003–$0.015 total
    const solPrice = state.solPriceUsd || 65;
    const estimatedTxCostUsd = 2 * 0.000005 * solPrice + (0.001 * solPrice); // base + priority
    const netProfitUsd = grossProfitUsd - estimatedTxCostUsd;

    // Determine execution mints: buy on cheaperDex (sell base for quote), receive quote
    // Then sell quote back on dearerDex to get more base back
    const inputMint = opp.baseMint;
    const outputMint = opp.quoteMint;
    const amountInAtomic = uiToAtomic(this.tradeAmountUi, opp.baseDecimals).toString();

    // Use the better quote's outAmount as expected output
    const bestQuote = opp.dearerDex === 'raydium' ? opp.raydiumQuote : opp.orcaQuote;
    const expectedOutAtomic = bestQuote?.outAmount ?? '0';

    const base: TradeDecision = {
      strategyId: this.id,
      pairLabel: opp.pairLabel,
      inputMint,
      outputMint,
      amountInAtomic,
      expectedOutAtomic,
      netProfitUsd,
      grossSpreadBps: opp.spreadBps,
      routeQualityScore: opp.spreadBps > 0 ? Math.min(1, opp.spreadBps / 100) : 0,
      freshnessScore: 1,
      metadata: {
        spreadBps: opp.spreadBps,
        netSpreadBps: opp.netSpreadBps,
        cheaperDex: opp.cheaperDex,
        dearerDex: opp.dearerDex,
        raydiumPriceOut: opp.raydiumPriceOut,
        orcaPriceOut: opp.orcaPriceOut,
        isPumpPair,
        notionalUsd,
        estimatedTxCostUsd,
        executionNote: 'requires_jito_bundle_two_tx',
      },
    };

    // Apply rejection gates
    if (opp.spreadBps < spreadThreshold) {
      base.rejectionReason = `spread_below_threshold:${opp.spreadBps}bps<${spreadThreshold}bps`;
    } else if (opp.netSpreadBps <= 0) {
      base.rejectionReason = `spread_eaten_by_fees:${opp.netSpreadBps}bps`;
    } else if (opp.cheaperDex === 'equal') {
      base.rejectionReason = 'no_spread:venues_equal';
    } else if (netProfitUsd < ctx.minProfitUsd) {
      base.rejectionReason = `below_min_profit:$${netProfitUsd.toFixed(4)}`;
    } else if (!opp.raydiumQuote || !opp.orcaQuote) {
      base.rejectionReason = `missing_venue_quote:raydium=${!!opp.raydiumQuote}_orca=${!!opp.orcaQuote}`;
    }

    return base;
  }

  /** Rotate through core pairs in batches. */
  private nextCoreBatch(): CrossDexScanPair[] {
    const n = Math.min(this.cfg.pairsPerScan, CROSS_DEX_CORE_PAIRS.length);
    const start = this.corePairIndex % CROSS_DEX_CORE_PAIRS.length;
    const batch: CrossDexScanPair[] = [];
    for (let i = 0; i < n; i++) {
      batch.push(CROSS_DEX_CORE_PAIRS[(start + i) % CROSS_DEX_CORE_PAIRS.length]!);
    }
    this.corePairIndex = (start + n) % CROSS_DEX_CORE_PAIRS.length;
    return batch;
  }
}
