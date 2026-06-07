import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { PairRegistry } from '../market/pair-registry.js';
import type { ScannableStrategy } from './types.js';
import type { StrategyContext, TradeDecision } from '../strategy/types.js';
export interface RouteDivergenceConfig {
    minDivergenceBps: number;
    minSurvivingEdgeBps: number;
    pairsPerScan: number;
    scanConcurrency: number;
}
export declare const DEFAULT_ROUTE_DIVERGENCE_CONFIG: RouteDivergenceConfig;
/**
 * Route divergence arb — compare restricted vs unrestricted route constructions
 * across CMC top-100 Solana pairs (rotating batch per scan).
 */
export declare class RouteDivergenceArbStrategy implements ScannableStrategy {
    private readonly client;
    private readonly tradeAmountUi;
    private readonly pairRegistry;
    private readonly cfg;
    private readonly paperMode;
    /**
     * liveQuotes=true: use real Jupiter quote API even in paper mode.
     * Measures actual route divergence on mainnet before committing capital.
     * Execution remains simulated when paperMode=true.
     */
    private readonly liveQuotes;
    readonly id = "route_divergence_arb";
    private scanTick;
    constructor(client: JupiterClient, tradeAmountUi: number, pairRegistry?: PairRegistry | null, cfg?: RouteDivergenceConfig, paperMode?: boolean, 
    /**
     * liveQuotes=true: use real Jupiter quote API even in paper mode.
     * Measures actual route divergence on mainnet before committing capital.
     * Execution remains simulated when paperMode=true.
     */
    liveQuotes?: boolean);
    scan(state: MarketState): Promise<MarketState>;
    evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null;
    private evaluateOne;
}
