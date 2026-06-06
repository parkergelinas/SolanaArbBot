import type { MarketState } from '../market/state.js';
import type { Strategy, StrategyContext, TradeDecision } from '../strategy/types.js';
import { SOL_MINT, USDC_MINT } from '../config/env.js';

export interface MeanReversionConfig {
  enabled: boolean;
  windowSize: number;
  entryZScore: number;
  exitZScore: number;
}

export const DEFAULT_MEAN_REVERSION_CONFIG: MeanReversionConfig = {
  enabled: false,
  windowSize: 20,
  entryZScore: 2.0,
  exitZScore: 0.5,
};

interface SpreadSample {
  ts: number;
  ratio: number;
}

/**
 * Pairs trading / mean reversion scaffold — rolling spread z-score on curated pairs.
 * Full execution wiring deferred; evaluates signals for observability when enabled.
 */
export class MeanReversionStrategy implements Strategy {
  readonly id = 'pairs_mean_reversion';
  private history: SpreadSample[] = [];

  constructor(private readonly cfg: MeanReversionConfig = DEFAULT_MEAN_REVERSION_CONFIG) {}

  evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    const sol = state.pricesUsd[SOL_MINT];
    const usdc = state.pricesUsd[USDC_MINT] ?? 1;
    if (!sol || sol <= 0) return null;

    const ratio = sol / usdc;
    this.history.push({ ts: state.timestampMs, ratio });
    if (this.history.length > this.cfg.windowSize) {
      this.history = this.history.slice(-this.cfg.windowSize);
    }

    if (this.history.length < this.cfg.windowSize) {
      return {
        strategyId: this.id,
        pairLabel: 'SOL/USDC',
        inputMint: SOL_MINT,
        outputMint: USDC_MINT,
        amountInAtomic: '0',
        expectedOutAtomic: '0',
        netProfitUsd: 0,
        grossSpreadBps: 0,
        routeQualityScore: 0,
        freshnessScore: 0,
        rejectionReason: 'warming_up',
        metadata: { samples: this.history.length, zScore: 0 },
      };
    }

    const mean = this.history.reduce((s, h) => s + h.ratio, 0) / this.history.length;
    const variance =
      this.history.reduce((s, h) => s + (h.ratio - mean) ** 2, 0) / this.history.length;
    const std = Math.sqrt(variance) || 1e-9;
    const zScore = (ratio - mean) / std;

    const spreadBps = Math.round(((ratio - mean) / mean) * 10_000);

    if (!this.cfg.enabled) {
      return {
        strategyId: this.id,
        pairLabel: 'SOL/USDC',
        inputMint: zScore > 0 ? SOL_MINT : USDC_MINT,
        outputMint: zScore > 0 ? USDC_MINT : SOL_MINT,
        amountInAtomic: '0',
        expectedOutAtomic: '0',
        netProfitUsd: 0,
        grossSpreadBps: Math.abs(spreadBps),
        routeQualityScore: 0.5,
        freshnessScore: 1,
        rejectionReason: 'mean_reversion_scaffold',
        metadata: { zScore, mean, ratio, windowSize: this.cfg.windowSize },
      };
    }

    if (Math.abs(zScore) < this.cfg.entryZScore) {
      return {
        strategyId: this.id,
        pairLabel: 'SOL/USDC',
        inputMint: SOL_MINT,
        outputMint: USDC_MINT,
        amountInAtomic: '0',
        expectedOutAtomic: '0',
        netProfitUsd: 0,
        grossSpreadBps: Math.abs(spreadBps),
        routeQualityScore: 0.5,
        freshnessScore: 1,
        rejectionReason: 'zscore_below_entry',
        metadata: { zScore, entryZScore: this.cfg.entryZScore },
      };
    }

    const sellSol = zScore > 0;
    return {
      strategyId: this.id,
      pairLabel: 'SOL/USDC',
      inputMint: sellSol ? SOL_MINT : USDC_MINT,
      outputMint: sellSol ? USDC_MINT : SOL_MINT,
      amountInAtomic: '0',
      expectedOutAtomic: '0',
      netProfitUsd: Math.abs(zScore) * 0.01 * ctx.minProfitUsd,
      grossSpreadBps: Math.abs(spreadBps),
      routeQualityScore: 0.6,
      freshnessScore: 1,
      rejectionReason: 'mean_reversion_not_wired',
      metadata: { zScore, direction: sellSol ? 'short_spread' : 'long_spread' },
    };
  }
}
