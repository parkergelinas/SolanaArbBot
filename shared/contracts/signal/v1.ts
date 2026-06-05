/**
 * Live signal contract v1 — normalized signals shared across services.
 * Rust mirror: `crates/signal-bus/src/types.rs`
 */

export const LIVE_SIGNAL_SCHEMA_VERSION = 1;

export type SignalKind = 'swap' | 'whale_alert' | 'smart_money_alert' | 'engine';

export type AlertType = 'whale_flow' | 'smart_money' | 'momentum';

export type StrategyTag = 'whale_copy_candidate' | 'watch_only' | 'informational';

export interface SignalSourceMeta {
  layer: string;
  dex: string;
  slot: number;
  wallet_label?: string;
}

/** Normalized live signal — swaps and alerts share one schema. */
export interface LiveSignal {
  v: typeof LIVE_SIGNAL_SCHEMA_VERSION;
  signal_id: string;
  kind: SignalKind;
  source: SignalSourceMeta;
  pair: string;
  token_in: string;
  token_out: string;
  timestamp_ms: number;
  tx_id: string;
  price: number;
  size: number;
  confidence: number;
  wallet: string;
  strength?: number;
  size_usd?: number;
  alert_type?: AlertType;
  strategy_tag?: StrategyTag;
  explanation?: string;
  direction?: string;
  /** Dedup key — signature or alert id. */
  dedup_key: string;
}
