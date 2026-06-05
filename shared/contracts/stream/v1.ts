/**
 * Stream contract v1 — canonical wire types for Rust ↔ TypeScript.
 * Bump `SCHEMA_VERSION` and add `stream/v2.ts` for breaking changes.
 */

export const SCHEMA_VERSION = 1;

export type Dex = 'raydium' | 'orca' | 'jupiter';

export type CandleInterval = '1s' | '5s' | '1m';

export interface SwapEvent {
  v: typeof SCHEMA_VERSION;
  signature: string;
  dex: Dex;
  token_in: string;
  token_out: string;
  amount_in: string;
  amount_out: string;
  wallet: string;
  slot: number;
  timestamp_ms: number;
}

export interface TokenPrice {
  v: typeof SCHEMA_VERSION;
  mint: string;
  price_usd: number;
  slot: number;
  timestamp_ms: number;
}

export interface Candle {
  v: typeof SCHEMA_VERSION;
  mint: string;
  interval: CandleInterval;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  ts_open_ms: number;
}

export type SignalKind = 'momentum' | 'whale_flow' | 'smart_money' | 'imbalance';

export interface Signal {
  v: typeof SCHEMA_VERSION;
  signal_id: string;
  mint: string;
  kind: SignalKind;
  strength: number;
  confidence: number;
  timestamp_ms: number;
  detail?: string;
}

/** Discriminated union — every WebSocket payload item. */
export type WSMessage =
  | { type: 'swap'; payload: SwapEvent }
  | { type: 'token_price'; payload: TokenPrice }
  | { type: 'candle'; payload: Candle }
  | { type: 'signal'; payload: Signal };

/** Batched frame emitted every 25–50 ms on `/stream`. */
export interface WSBatchFrame {
  v: typeof SCHEMA_VERSION;
  type: 'batch';
  seq: number;
  ts_ms: number;
  messages: WSMessage[];
}

export function isWSBatchFrame(msg: unknown): msg is WSBatchFrame {
  return (
    typeof msg === 'object' &&
    msg !== null &&
    (msg as WSBatchFrame).type === 'batch' &&
    (msg as WSBatchFrame).v === SCHEMA_VERSION &&
    Array.isArray((msg as WSBatchFrame).messages)
  );
}

export function isWSMessage(msg: unknown): msg is WSMessage {
  if (typeof msg !== 'object' || msg === null) return false;
  const t = (msg as WSMessage).type;
  return (
    t === 'swap' ||
    t === 'token_price' ||
    t === 'candle' ||
    t === 'signal'
  );
}
