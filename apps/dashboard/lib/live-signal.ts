import type { LiveSignal } from '../../../shared/contracts/signal/v1';
import type { Direction, SignalEvent, SignalType } from './types';

const EMPTY_FEATURES = {
  volume_short: 0,
  volume_long: 0,
  price_velocity: 0,
  liquidity_delta_pct: 0,
  whale_activity_score: 0,
  smart_money_score: 0,
  data_points: 1,
};

/** Matches `signal_bus::alert_id_hash` for dedup with control-api DTOs. */
export function signalNumericId(id: string): number {
  let h = 0;
  for (let i = 0; i < id.length; i++) {
    h = Math.imul(31, h) + id.charCodeAt(i);
    h |= 0;
  }
  const u = h >>> 0;
  if (h > 0) {
    return Number(BigInt('18446744073709551616') - BigInt(u) + BigInt(1));
  }
  return u;
}

function kindToSignalType(s: LiveSignal): SignalType {
  if (s.alert_type === 'whale_flow') return 'WhaleFlow';
  if (s.alert_type === 'smart_money') return 'SmartMoney';
  if (s.alert_type === 'momentum') return 'Momentum';
  switch (s.kind) {
    case 'whale_alert':
      return 'WhaleFlow';
    case 'smart_money_alert':
      return 'SmartMoney';
    case 'swap':
      return 'Swap';
    default:
      return 'Momentum';
  }
}

export function liveSignalToEvent(s: LiveSignal): SignalEvent {
  const strength = s.strength ?? s.confidence;
  const signal_type = kindToSignalType(s);
  const fv = { ...EMPTY_FEATURES, volume_short: s.size };
  if (signal_type === 'WhaleFlow') fv.whale_activity_score = strength;
  if (signal_type === 'SmartMoney') fv.smart_money_score = s.confidence;

  return {
    signal_id: signalNumericId(s.signal_id),
    timestamp_micros: s.timestamp_ms * 1000,
    pool_address: s.token_out || s.pair,
    signal_type,
    strength,
    confidence: s.confidence,
    direction: (s.direction ?? 'Long') as Direction,
    timeframe_secs: 300,
    feature_vector: fv,
    explanation:
      s.explanation ?? `${s.pair} · ${s.source.dex} · ${s.tx_id.slice(0, 8)}…`,
  };
}

export function liveSignalsToEvents(signals: LiveSignal[]): SignalEvent[] {
  return signals.map(liveSignalToEvent);
}
