import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { PairRegistry } from '../market/pair-registry.js';
import type { ScannableStrategy } from '../signals/types.js';
import type { StrategyContext, TradeDecision } from './types.js';
export interface RoundTripConfig {
    pairsPerScan: number;
    scanConcurrency: number;
}
export declare const DEFAULT_ROUND_TRIP_CONFIG: RoundTripConfig;
/** Round-trip quote arbitrage across CMC top-100 Solana pairs. */
export declare class RoundTripQuoteArbStrategy implements ScannableStrategy {
    private readonly client;
    private readonly tradeAmountUi;
    private readonly pairRegistry;
    private readonly cfg;
    readonly id = "round_trip_quote_arb";
    constructor(client: JupiterClient, tradeAmountUi: number, pairRegistry?: PairRegistry | null, cfg?: RoundTripConfig);
    scan(state: MarketState): Promise<MarketState>;
    evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null;
    private evaluateOne;
}
