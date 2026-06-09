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

// ── New strategies (v2) ───────────────────────────────────────────────────────
export { LstSpreadArbStrategy, DEFAULT_LST_CONFIG, LST_SCAN_PAIRS } from '../strategy/lst-spread-arb.js';
export { TriangleArbStrategy, DEFAULT_TRIANGLE_CONFIG, TRIANGLE_CYCLES } from '../strategy/triangle-arb.js';
export { StableDepegStrategy, DEFAULT_STABLE_CONFIG, STABLE_PAIRS } from '../strategy/stable-depeg.js';
export { MomentumLagStrategy, DEFAULT_MOMENTUM_CONFIG, MOMENTUM_PAIRS } from '../strategy/momentum-lag.js';
export { RouteDiversitySweepStrategy, DEFAULT_DIVERSITY_CONFIG, DIVERSITY_PAIRS } from '../strategy/route-diversity-sweep.js';
export type {
  Strategy,
  StrategyContext,
  TradeDecision,
} from './types.js';
