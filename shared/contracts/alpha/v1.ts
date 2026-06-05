/**
 * Alpha engine contract v1 — fusion, scoring, trade signals.
 */

export const ALPHA_SCHEMA_VERSION = 1;

export interface WalletScore {
  wallet: string;
  score: number;
  confidence: number;
  token: string;
  timestamp: number;
}

export interface MarketSignal {
  token: string;
  momentum: number;
  volume_spike: number;
  price_change: number;
  timestamp: number;
}

export interface MicrostructureEvent {
  token: string;
  imbalance: number;
  momentum: number;
  volatility: number;
  timestamp: number;
}

export interface ArbSignal {
  token_pair: string;
  spread_pct: number;
  confidence: number;
}

export interface FusedFeatureVector {
  token: string;
  wallet_smart_money_score: number;
  momentum_strength: number;
  liquidity_conditions: number;
  arbitrage_opportunity_strength: number;
  volume_spike_norm: number;
  price_change: number;
  imbalance: number;
  timestamp: number;
  ttl_ms: number;
}

export interface ScoringWeights {
  w1: number;
  w2: number;
  w3: number;
  w4: number;
  w5: number;
}

export interface AlphaSignal {
  signal_id: string;
  token_in: string;
  token_out: string;
  wallet: string;
  confidence: number;
  expected_edge: number;
  size_usd: number;
  strategy: string;
  score: number;
  direction: string;
  timestamp: number;
}

export interface TradeSignal {
  v: typeof ALPHA_SCHEMA_VERSION;
  token: string;
  direction: 'long' | 'short' | string;
  size_usd: number;
  confidence: number;
  expected_edge: number;
  strategy: string;
  timestamp_ms: number;
}
