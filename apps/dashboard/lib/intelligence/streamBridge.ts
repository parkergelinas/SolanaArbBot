import type { Signal, SignalKind } from '@/lib/stream/types';
import type { Direction, SignalEvent, SignalType } from '@/lib/types';

/** Stable negative id from stream signal_id string. */
function streamSignalId(signalId: string): number {
  let h = 0;
  for (let i = 0; i < signalId.length; i++) {
    h = (h * 31 + signalId.charCodeAt(i)) | 0;
  }
  const id = h > 0 ? -h - 1_000_000 : h - 1_000_000;
  return id;
}

const KIND_TO_TYPE: Record<SignalKind, SignalType> = {
  whale_flow: 'WhaleFlow',
  smart_money: 'SmartMoney',
  momentum: 'Momentum',
  imbalance: 'Momentum',
  arb: 'Momentum',
  route_divergence: 'Momentum',
};

const EMPTY_FEATURES = {
  volume_short: 0,
  volume_long: 0,
  price_velocity: 0,
  liquidity_delta_pct: 0,
  whale_activity_score: 0,
  smart_money_score: 0,
  data_points: 1,
};

function directionForKind(kind: SignalKind, strength: number): Direction {
  if (kind === 'imbalance') {
    return strength >= 0.55 ? 'Long' : strength <= 0.45 ? 'Short' : 'Neutral';
  }
  if (kind === 'whale_flow' || kind === 'smart_money') return 'Long';
  return strength >= 0.6 ? 'Long' : strength <= 0.4 ? 'Short' : 'Neutral';
}

export function streamSignalToEvent(sig: Signal): SignalEvent {
  const signalType = KIND_TO_TYPE[sig.kind];
  const features = { ...EMPTY_FEATURES };

  if (signalType === 'WhaleFlow') {
    features.whale_activity_score = sig.strength;
    features.volume_short = sig.strength * 10;
  } else if (signalType === 'SmartMoney') {
    features.smart_money_score = sig.confidence;
  } else {
    features.price_velocity = sig.strength - 0.5;
  }

  const detail = sig.detail ?? `${sig.kind.replace('_', ' ')} on ${sig.mint.slice(0, 8)}`;

  return {
    signal_id: streamSignalId(sig.signal_id),
    timestamp_micros: sig.timestamp_ms * 1000,
    pool_address: sig.mint,
    signal_type: signalType,
    strength: sig.strength,
    confidence: sig.confidence,
    direction: directionForKind(sig.kind, sig.strength),
    timeframe_secs: 300,
    feature_vector: features,
    explanation: `[stream-api] ${detail}`,
  };
}

export function streamSignalsToEvents(signals: Signal[]): SignalEvent[] {
  return signals
    .map(streamSignalToEvent)
    .sort((a, b) => b.timestamp_micros - a.timestamp_micros);
}
