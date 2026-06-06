import type { MarketState } from '../market/state.js';
import type { Strategy, StrategyContext, TradeDecision } from '../strategy/types.js';
export interface MeanReversionConfig {
    enabled: boolean;
    windowSize: number;
    entryZScore: number;
    exitZScore: number;
}
export declare const DEFAULT_MEAN_REVERSION_CONFIG: MeanReversionConfig;
/**
 * Pairs trading / mean reversion scaffold — rolling spread z-score on curated pairs.
 * Full execution wiring deferred; evaluates signals for observability when enabled.
 */
export declare class MeanReversionStrategy implements Strategy {
    private readonly cfg;
    readonly id = "pairs_mean_reversion";
    private history;
    constructor(cfg?: MeanReversionConfig);
    evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null;
}
