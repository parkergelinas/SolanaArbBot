/**
 * Arbitrage engine contract v1 — pool prices and arb signals.
 */

export const ARB_SCHEMA_VERSION = 1;

export interface PoolPrice {
  dex: string;
  token_a: string;
  token_b: string;
  price: number;
  liquidity: number;
  timestamp: number;
}

export interface ArbSignal {
  v: typeof ARB_SCHEMA_VERSION;
  token_pair: string;
  spread_pct: number;
  confidence: number;
}
