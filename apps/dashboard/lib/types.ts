// ─── Core domain types (mirror of Rust DTOs) ────────────────────────────────

export type SignalType = 'WhaleFlow' | 'SmartMoney' | 'Momentum';
export type Direction  = 'Long' | 'Short' | 'Neutral';

export interface FeatureVector {
  volume_short:          number;
  volume_long:           number;
  price_velocity:        number;
  liquidity_delta_pct:   number;
  whale_activity_score:  number;
  smart_money_score:     number;
  data_points:           number;
}

export interface SignalEvent {
  signal_id:        number;
  timestamp_micros: number;
  pool_address:     string;
  signal_type:      SignalType;
  strength:         number;
  confidence:       number;
  direction:        Direction;
  timeframe_secs:   number;
  feature_vector:   FeatureVector;
  explanation:      string;
}

export interface HealthStatus {
  status:          string;
  uptime_secs:     number;
  signals_stored:  number;
  version:         string;
}

export interface SystemStatus {
  running:           boolean;
  mode:              string;
  signals_processed: number;
  events_processed:  number;
  last_signal_ts:    number | null;
}

export interface Portfolio {
  capital_usd:      number;
  unrealised_pnl:   number;
  realised_pnl:     number;
  open_positions:   number;
  total_trades:     number;
  win_rate:         number;
}

export interface Risk {
  capital_usd:           number;
  max_position_pct:      number;
  max_drawdown_pct:      number;
  current_exposure_pct:  number;
  daily_loss_usd:        number;
  risk_status:           string;
}

export interface CommandResult {
  success: boolean;
  message: string;
}

export interface BotStatus {
  running: boolean;
  mode: string;
  scalp_trades: number;
  arb_trades: number;
  scalp_pnl_usd: number;
  arb_pnl_usd: number;
  net_pnl_usd: number;
  total_trades: number;
  win_rate: number;
  risk_status: string;
  risk_state: string;
  trading_halted: boolean;
  halt_reason?: string | null;
  scalp_enabled: boolean;
  arb_enabled: boolean;
  daily_loss_usd: number;
}

// ─── WebSocket event envelope ────────────────────────────────────────────────

export type WsEvent =
  | { type: 'signal';       data: SignalEvent     }
  | { type: 'health';       data: HealthStatus    }
  | { type: 'status';       data: SystemStatus    }
  | { type: 'portfolio';    data: Portfolio       }
  | { type: 'risk';         data: Risk            }
  | { type: 'config_changed'; data: { section: string; summary: string } };

/** Batched WebSocket frame from control-api stream engine */
export interface WsBatchFrame {
  type: 'batch';
  seq: number;
  ts_micros: number;
  events: WsEvent[];
}

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

// ─── Helpers ─────────────────────────────────────────────────────────────────

export function formatPct(v: number, decimals = 1) {
  return `${(v * 100).toFixed(decimals)}%`;
}

export function formatUsd(v: number) {
  return v.toLocaleString('en-US', { style: 'currency', currency: 'USD', maximumFractionDigits: 2 });
}

export function tsToDate(micros: number) {
  return new Date(micros / 1000);
}

export function signalColor(type: SignalType) {
  return type === 'WhaleFlow' ? '#3b82f6' : type === 'SmartMoney' ? '#a855f7' : '#22c55e';
}

export function directionColor(dir: Direction) {
  return dir === 'Long' ? '#22c55e' : dir === 'Short' ? '#ef4444' : '#94a3b8';
}
