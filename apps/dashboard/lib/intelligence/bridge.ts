import type { SignalEvent } from '../types';
import type { SmartMoneyAlert, WhaleAlert } from './types';

/** Stable negative id from alert string (avoids collision with engine signal ids). */
function alertId(alertId: string): number {
  let h = 0;
  for (let i = 0; i < alertId.length; i++) {
    h = (h * 31 + alertId.charCodeAt(i)) | 0;
  }
  return h > 0 ? -h : h;
}

const EMPTY_FEATURES = {
  volume_short: 0,
  volume_long: 0,
  price_velocity: 0,
  liquidity_delta_pct: 0,
  whale_activity_score: 0,
  smart_money_score: 0,
  data_points: 1,
};

export function whaleToSignal(alert: WhaleAlert): SignalEvent {
  return {
    signal_id: alertId(alert.alert_id),
    timestamp_micros: alert.timestamp * 1000,
    pool_address: alert.token,
    signal_type: 'WhaleFlow',
    strength: alert.strength,
    confidence: alert.confidence,
    direction: 'Long',
    timeframe_secs: 300,
    feature_vector: {
      ...EMPTY_FEATURES,
      whale_activity_score: alert.strength,
      volume_short: alert.amount_sol,
    },
    explanation:
      `[intelligence-api] ${alert.detail ?? `Whale ${alert.amount_sol.toFixed(2)} SOL ($${alert.notional_usd.toFixed(0)}) on ${alert.dex} · ${alert.token_symbol}`} · tier ${alert.tier}`,
  };
}

export function smartMoneyToSignal(alert: SmartMoneyAlert): SignalEvent {
  return {
    signal_id: alertId(alert.alert_id),
    timestamp_micros: alert.timestamp * 1000,
    pool_address: alert.token,
    signal_type: 'SmartMoney',
    strength: alert.strength,
    confidence: alert.confidence,
    direction: 'Long',
    timeframe_secs: 300,
    feature_vector: {
      ...EMPTY_FEATURES,
      smart_money_score: alert.confidence,
      volume_short: alert.amount_sol,
    },
    explanation:
      `[intelligence-api] ${alert.detail ?? `Smart money ${alert.amount_sol.toFixed(2)} SOL on ${alert.dex} · ${alert.token_symbol}`}`,
  };
}

export function intelligenceToSignals(
  whales: WhaleAlert[],
  smart: SmartMoneyAlert[],
): SignalEvent[] {
  return [
    ...whales.map(whaleToSignal),
    ...smart.map(smartMoneyToSignal),
  ].sort((a, b) => b.timestamp_micros - a.timestamp_micros);
}
