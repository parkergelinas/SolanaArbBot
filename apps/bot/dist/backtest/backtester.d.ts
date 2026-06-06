import type { QuotePairSnapshot } from '../jupiter/types.js';
import type { MarketState } from '../market/state.js';
export interface BacktestSimConfig {
    quoteStalenessMs: number;
    latencyDriftMs: number;
    landingSlippageBps: number;
    missedFillRate: number;
    priorityFeeLamports: number;
    routeChangePenaltyBps: number;
}
export declare const DEFAULT_BACKTEST_SIM: BacktestSimConfig;
export interface BacktestQuoteRecord {
    pair: QuotePairSnapshot;
    state: MarketState;
    inputDecimals: number;
}
export interface BacktestResult {
    opportunities: number;
    simulatedFills: number;
    missedFills: number;
    totalExpectedUsd: number;
    totalSimulatedUsd: number;
    avgEdgeBps: number;
}
/**
 * Replay quote pairs with execution economics — not just raw spread counting.
 */
export declare function runBacktest(records: BacktestQuoteRecord[], sim?: BacktestSimConfig): BacktestResult;
