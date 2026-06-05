/**
 * Canonical stream contracts for the control-plane WebSocket.
 * Keep in sync with:
 *   - apps/control-api/src/events.rs
 *   - apps/dashboard/lib/types.ts (re-exports from here in future)
 */

export type {
  TradeEvent,
  TradeMode,
  TradeSide,
  TradeStage,
} from './trade/v1';
export { isTerminalTradeStage, TERMINAL_TRADE_STAGES, TRADE_SCHEMA_VERSION } from './trade/v1';

export type SignalType = 'WhaleFlow' | 'SmartMoney' | 'Momentum';
export type Direction = 'Long' | 'Short' | 'Neutral';

export interface FeatureVector {
  volume_short: number;
  volume_long: number;
  price_velocity: number;
  liquidity_delta_pct: number;
  whale_activity_score: number;
  smart_money_score: number;
  data_points: number;
}

export interface SignalEvent {
  signal_id: number;
  timestamp_micros: number;
  pool_address: string;
  signal_type: SignalType | 'Swap';
  strength: number;
  confidence: number;
  direction: Direction;
  timeframe_secs: number;
  feature_vector: FeatureVector;
  explanation: string;
  /** `engine` | `intelligence` | `data-layer` */
  source?: string;
  /** `whale_copy_candidate` | `watch_only` | `informational` */
  strategy_tag?: string;
  wallet?: string;
  size_usd?: number;
  size_sol?: number;
}

export interface HealthStatus {
  status: string;
  uptime_secs: number;
  signals_stored: number;
  version: string;
}

export interface SystemStatus {
  running: boolean;
  mode: string;
  signals_processed: number;
  events_processed: number;
  last_signal_ts: number | null;
}

export interface Portfolio {
  capital_usd: number;
  unrealised_pnl: number;
  realised_pnl: number;
  open_positions: number;
  total_trades: number;
  win_rate: number;
}

export interface Risk {
  capital_usd: number;
  max_position_pct: number;
  max_drawdown_pct: number;
  current_exposure_pct: number;
  daily_loss_usd: number;
  risk_status: string;
}

/** Single domain event (internal tagged union). */
export type WsEvent =
  | { type: 'signal'; data: SignalEvent }
  | { type: 'trade'; data: TradeEvent }
  | { type: 'health'; data: HealthStatus }
  | { type: 'status'; data: SystemStatus }
  | { type: 'portfolio'; data: Portfolio }
  | { type: 'risk'; data: Risk }
  | { type: 'config_changed'; data: { section: string; summary: string } };

/** Wire frame: batched events flushed every 25–50 ms on the server. */
export interface WsBatchFrame {
  type: 'batch';
  seq: number;
  ts_micros: number;
  events: WsEvent[];
}

export const STREAM_BATCH_MS_DEFAULT = 33;

export function isWsBatchFrame(msg: unknown): msg is WsBatchFrame {
  return (
    typeof msg === 'object' &&
    msg !== null &&
    (msg as WsBatchFrame).type === 'batch' &&
    Array.isArray((msg as WsBatchFrame).events)
  );
}

export function isWsEvent(msg: unknown): msg is WsEvent {
  return (
    typeof msg === 'object' &&
    msg !== null &&
    typeof (msg as WsEvent).type === 'string' &&
    'data' in (msg as object)
  );
}
