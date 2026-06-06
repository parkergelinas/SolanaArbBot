export type { FeeBreakdownUsd, ScoringInput, ScoringResult, Strategy, StrategyContext, TradeDecision, TradeSide, } from '../strategy/types.js';
/** Strategies that fetch quotes asynchronously before `evaluate`. */
export interface ScannableStrategy {
    readonly id: string;
    scan(state: import('../market/state.js').MarketState): Promise<import('../market/state.js').MarketState>;
    evaluate(state: import('../market/state.js').MarketState, ctx: import('../strategy/types.js').StrategyContext): import('../strategy/types.js').TradeDecision | null;
}
