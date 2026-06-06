/** Signal layer — pluggable strategies producing TradeDecision from MarketState. */
export { StrategyRegistry } from '../strategy/registry.js';
export { RoundTripQuoteArbStrategy } from '../strategy/round-trip-arb.js';
export { RouteDivergenceArbStrategy, DEFAULT_ROUTE_DIVERGENCE_CONFIG } from './route-divergence-arb.js';
export { MeanReversionStrategy, DEFAULT_MEAN_REVERSION_CONFIG } from './mean-reversion.js';
export type { RouteDivergenceConfig } from './route-divergence-arb.js';
export type { MeanReversionConfig } from './mean-reversion.js';
export type { ScannableStrategy } from './types.js';
export type { Strategy, StrategyContext, TradeDecision, } from './types.js';
