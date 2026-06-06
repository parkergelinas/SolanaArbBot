import type { MarketState } from '../market/state.js';
import type { QuotePairSnapshot } from '../jupiter/types.js';
export type TradeSide = 'buy' | 'sell' | 'round_trip';
export interface FeeBreakdownUsd {
    priorityFeeUsd: number;
    jitoTipUsd: number;
    slippageUsd: number;
    totalUsd: number;
}
export interface TradeDecision {
    strategyId: string;
    pairLabel: string;
    inputMint: string;
    outputMint: string;
    amountInAtomic: string;
    expectedOutAtomic: string;
    /** Expected net profit after all costs — always USD. */
    netProfitUsd: number;
    grossSpreadBps: number;
    routeQualityScore: number;
    freshnessScore: number;
    rejectionReason?: string;
    metadata?: Record<string, unknown>;
}
export interface StrategyContext {
    minProfitUsd: number;
    slippageBps: number;
}
/** Pluggable strategy contract — add route-arb, mean reversion, momentum, triggers later. */
export interface Strategy {
    readonly id: string;
    evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null;
}
export interface ScoringInput {
    pair: QuotePairSnapshot;
    state: MarketState;
    inputDecimals: number;
    outputDecimals: number;
    feesUsd: FeeBreakdownUsd;
}
export interface ScoringResult {
    netProfitUsd: number;
    grossSpreadBps: number;
    startUsd: number;
    endUsd: number;
    rejected: boolean;
    rejectionReason?: string;
}
