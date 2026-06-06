import { SCHEMA_VERSION, type Signal, type SignalKind } from '@/lib/stream/types';
import type { SignalEvent, SignalType } from '@/lib/types';

const KIND_MAP: Partial<Record<SignalType, SignalKind>> = {
  Momentum: 'momentum',
  WhaleFlow: 'whale_flow',
  SmartMoney: 'smart_money',
  Swap: 'momentum',
};

/** Map control-api SignalEvent → stream wire Signal for terminal panel. */
export function signalEventToStreamSignal(e: SignalEvent): Signal {
  return {
    v: SCHEMA_VERSION,
    signal_id: String(e.signal_id),
    mint: e.pool_address,
    kind: KIND_MAP[e.signal_type] ?? 'momentum',
    strength: e.strength,
    confidence: e.confidence,
    timestamp_ms: Math.floor(e.timestamp_micros / 1000),
    detail: e.explanation,
  };
}
