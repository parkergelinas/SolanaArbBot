import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { ScannableStrategy } from './types.js';
import type { StrategyContext, TradeDecision } from '../strategy/types.js';
export interface RouteDivergenceConfig {
    minDivergenceBps: number;
    minSurvivingEdgeBps: number;
}
export declare const DEFAULT_ROUTE_DIVERGENCE_CONFIG: RouteDivergenceConfig;
/**
 * Route divergence arb — compare restricted vs unrestricted (and alternate)
 * route constructions; trade only when spread survives fees, latency, and staleness.
 */
export declare class RouteDivergenceArbStrategy implements ScannableStrategy {
    private readonly client;
    private readonly tradeAmountUi;
    private readonly cfg;
    readonly id = "route_divergence_arb";
    constructor(client: JupiterClient, tradeAmountUi: number, cfg?: RouteDivergenceConfig);
    scan(state: MarketState): Promise<MarketState>;
    evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null;
}
