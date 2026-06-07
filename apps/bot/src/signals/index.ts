/** Signal layer — pluggable strategies producing TradeDecision from MarketState. */

export { StrategyRegistry } from '../strategy/registry.js';
export { RoundTripQuoteArbStrategy, DEFAULT_ROUND_TRIP_CONFIG } from '../strategy/round-trip-arb.js';
export { RouteDivergenceArbStrategy, DEFAULT_ROUTE_DIVERGENCE_CONFIG } from './route-divergence-arb.js';
export { PumpEdgeStrategy } from './pump-edge.js';
export {
  MeanReversionStrategy,
  DEFAULT_MEAN_REVERSION_CONFIG,
} from './mean-reversion.js';
export {
  CrossDexArbStrategy,
  DEFAULT_CROSS_DEX_CONFIG,
  CROSS_DEX_CORE_PAIRS,
} from './cross-dex-arb.js';
export type { CrossDexArbConfig } from './cross-dex-arb.js';
export { AdaptiveArbStrategy, DEFAULT_ADAPTIVE_CONFIG } from './adaptive-arb.js';
export type { AdaptiveArbConfig } from './adaptive-arb.js';
export type { RouteDivergenceConfig } from './route-divergence-arb.js';
export type { MeanReversionConfig } from './mean-reversion.js';
export type { ScannableStrategy } from './types.js';
export type {
  Strategy,
  StrategyContext,
  TradeDecision,
} from './types.js';
