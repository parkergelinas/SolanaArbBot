import type { SignalEvent } from '../types';
import type { EnrichedSwapEvent, SmartMoneyAlert, SwapEvent, WhaleAlert } from './types';
import { isWhaleSwap } from './whaleSources';

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

export function swapToSignal(swap: SwapEvent | EnrichedSwapEvent): SignalEvent | null {
  const amountSol = swap.amount_sol;
  if (amountSol < 1) return null;

  const walletLabel =
    'wallet_label' in swap && swap.wallet_label ? swap.wallet_label : undefined;
  const symbol = 'token_symbol' in swap ? swap.token_symbol : swap.token.slice(0, 6);
  const notional =
    'notional_usd' in swap && swap.notional_usd > 0
      ? swap.notional_usd
      : amountSol * 150;

  const isWhale = isWhaleSwap(amountSol, walletLabel ?? undefined);
  const signalType = isWhale ? 'WhaleFlow' : amountSol >= 5 ? 'Momentum' : 'Swap';
  const sig = swap.signature;

  return {
    signal_id: alertId(`swap_${sig}`),
    timestamp_micros: swap.timestamp * 1000,
    pool_address: swap.token,
    signal_type: signalType,
    strength: Math.min(1, amountSol / 20),
    confidence: isWhale ? 0.75 : 0.55,
    direction: 'Long',
    timeframe_secs: 120,
    feature_vector: {
      ...EMPTY_FEATURES,
      volume_short: amountSol,
      whale_activity_score: isWhale ? 0.8 : 0,
      price_velocity: amountSol / 100,
    },
    explanation: `[intelligence-api] ${amountSol.toFixed(2)} SOL ($${notional.toFixed(0)}) ${swap.dex} · ${symbol}${walletLabel ? ` · ${walletLabel}` : ''}`,
  };
}

export function intelligenceToSignals(
  whales: WhaleAlert[],
  smart: SmartMoneyAlert[],
  swaps: (SwapEvent | EnrichedSwapEvent)[] = [],
): SignalEvent[] {
  const swapSignals = swaps
    .map(swapToSignal)
    .filter((s): s is SignalEvent => s !== null);

  return [
    ...whales.map(whaleToSignal),
    ...smart.map(smartMoneyToSignal),
    ...swapSignals,
  ].sort((a, b) => b.timestamp_micros - a.timestamp_micros);
}
