import type { MarketState } from '../market/state.js';
import type { Strategy, StrategyContext, TradeDecision } from './types.js';
export declare class StrategyRegistry {
    private strategies;
    register(strategy: Strategy): this;
    list(): readonly Strategy[];
    evaluateAll(state: MarketState, ctx: StrategyContext): TradeDecision[];
    best(state: MarketState, ctx: StrategyContext): TradeDecision | null;
}
