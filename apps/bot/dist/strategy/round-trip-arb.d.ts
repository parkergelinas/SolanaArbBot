import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { Strategy, StrategyContext, TradeDecision } from './types.js';
/** Round-trip quote arbitrage — first pluggable strategy implementation. */
export declare class RoundTripQuoteArbStrategy implements Strategy {
    private readonly client;
    private readonly tradeAmountUi;
    readonly id = "round_trip_quote_arb";
    constructor(client: JupiterClient, tradeAmountUi: number);
    scan(state: MarketState): Promise<MarketState>;
    evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null;
}
