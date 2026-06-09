/**
 * Backtest engine — replays journal rejection events to find optimal adaptive
 * strategy parameters without requiring historical trade data.
 *
 * How it works:
 *  1. Extract real spread observations from rejection events
 *     (e.g. "spread_below_threshold:12bps<100bps" → observed 12 bps)
 *  2. Compute the break-even spread threshold for each capital/size combo
 *  3. Sweep minSpreadBps × minProfitUsd configs and simulate which would execute
 *  4. Return the config that maximises estimated net profit
 */
import type { JournalEvent } from '../state/journal.js';
export interface SpreadObservation {
    spreadBps: number;
    pairLabel: string;
    stage: string;
    tradeSizeUi: number;
    notionalUsd: number;
    grossProfitUsd: number;
    netProfitUsd: number;
}
export interface BacktestConfig {
    minSpreadBps: number;
    minProfitUsd: number;
    tradeSizeUi: number;
    solPriceUsd: number;
}
export interface BacktestResult {
    config: BacktestConfig;
    totalObservations: number;
    triggeredTrades: number;
    triggerRate: number;
    estimatedTotalProfitUsd: number;
    avgNetProfitPerTrade: number;
    profitableTrades: number;
    losingTrades: number;
    /** Annualised % return estimate (very rough — assumes same frequency). */
    estimatedAprPct: number;
}
export interface BacktestSweepResult {
    observations: SpreadObservation[];
    configs: BacktestResult[];
    best: BacktestResult | null;
    /** Break-even spread at given trade size and sol price. */
    breakEvenBps: number;
    /** Current stage's threshold vs break-even gap. */
    thresholdGapBps: number;
    recommendations: string[];
}
/** Extract real spread values from journal rejection reasons. */
export declare function extractSpreadObservations(events: readonly JournalEvent[], solPriceUsd?: number): SpreadObservation[];
/**
 * Full parameter sweep.
 *
 * @param events       In-memory journal events
 * @param tradeSizeUi  SOL trade size (default 0.5 = Stage 1)
 * @param solPriceUsd  Current SOL price
 * @param hoursOfData  How many hours of observation data (used for rate estimates)
 */
export declare function runBacktest(events: readonly JournalEvent[], tradeSizeUi?: number, solPriceUsd?: number, hoursOfData?: number): BacktestSweepResult;
