/**
 * Execution engine contract v1 — trade signals and order lifecycle.
 */

export const EXECUTION_SCHEMA_VERSION = 1;

export interface TradeSignal {
  v: typeof EXECUTION_SCHEMA_VERSION;
  token: string;
  direction: 'long' | 'short' | string;
  size_usd: number;
  confidence: number;
  expected_edge: number;
  strategy: string;
  timestamp_ms: number;
}

export type OrderStatus = 'pending' | 'submitted' | 'confirmed' | 'failed';

export interface OrderRecord {
  v: typeof EXECUTION_SCHEMA_VERSION;
  order_id: string;
  signal: TradeSignal;
  status: OrderStatus;
  tx_signature?: string | null;
  error?: string | null;
  created_at: number;
  updated_at: number;
}

export type LifecycleEvent =
  | { type: 'order_created'; payload: OrderRecord }
  | { type: 'order_updated'; payload: OrderRecord };
