/**
 * Trade lifecycle contract v1 — canonical TradeEvent for control-plane WS.
 * Mirror: apps/control-api/src/dto.rs (TradeEventDto)
 */

export const TRADE_SCHEMA_VERSION = 1;

export type TradeStage =
  | 'started'
  | 'quoted'
  | 'validated'
  | 'submitted'
  | 'filled'
  | 'failed'
  | 'rejected'
  | 'canceled';

export type TradeMode = 'paper' | 'live';

export type TradeSide = 'long' | 'short' | 'buy' | 'sell';

/** Terminal stages — no further lifecycle updates expected. */
export const TERMINAL_TRADE_STAGES: ReadonlySet<TradeStage> = new Set([
  'filled',
  'failed',
  'rejected',
  'canceled',
]);

export interface TradeEvent {
  v: typeof TRADE_SCHEMA_VERSION;
  trade_id: string;
  wallet_id: string;
  source_strategy: string;
  pair: string;
  side: TradeSide;
  size_usd: number;
  expected_pnl_usd: number;
  tx_signature: string | null;
  timestamp_us: number;
  stage: TradeStage;
  mode: TradeMode;
  reject_reason?: string | null;
  signal_id?: number | null;
}

export function isTerminalTradeStage(stage: TradeStage): boolean {
  return TERMINAL_TRADE_STAGES.has(stage);
}
