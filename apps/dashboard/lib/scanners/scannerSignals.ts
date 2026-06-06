import type { SignalEvent } from '@/lib/types';

import type { PumpMomentumHit, ShitcoinWhaleHit } from './types';

function scannerId(prefix: string, key: string): number {
  let h = 0;
  const s = `${prefix}_${key}`;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) | 0;
  }
  return h > 0 ? -h - 3_000_000 : h - 3_000_000;
}

const EMPTY = {
  volume_short: 0,
  volume_long: 0,
  price_velocity: 0,
  liquidity_delta_pct: 0,
  whale_activity_score: 0,
  smart_money_score: 0,
  data_points: 1,
};

function shortWallet(wallet: string): string {
  if (wallet.length <= 10) return wallet;
  return `${wallet.slice(0, 4)}…${wallet.slice(-4)}`;
}

export function solscanHitToSignal(hit: ShitcoinWhaleHit): SignalEvent {
  return {
    signal_id: scannerId('solscan', hit.wallet + hit.tokenMint),
    timestamp_micros: hit.detectedAtMs * 1000,
    pool_address: hit.tokenMint,
    signal_type: 'WhaleFlow',
    strength: Math.min(1, hit.score / 100),
    confidence: Math.min(0.9, 0.5 + hit.score / 200),
    direction: 'Long',
    timeframe_secs: 300,
    feature_vector: {
      ...EMPTY,
      volume_short: hit.amountUsd,
      whale_activity_score: hit.score / 100,
    },
    explanation: `[solscan/dexscreener] ${shortWallet(hit.wallet)} bought ${hit.tokenSymbol} · $${hit.amountUsd.toFixed(0)} · score ${hit.score} · mcap $${hit.marketCapUsd.toFixed(0)}`,
  };
}

export function pumpHitToSignal(hit: PumpMomentumHit): SignalEvent {
  return {
    signal_id: scannerId('pump', hit.mint + String(hit.detectedAtMs)),
    timestamp_micros: hit.detectedAtMs * 1000,
    pool_address: hit.mint,
    signal_type: 'Momentum',
    strength: Math.min(1, hit.momentumScore / 100),
    confidence: Math.min(0.85, 0.45 + hit.momentumScore / 150),
    direction: 'Long',
    timeframe_secs: 180,
    feature_vector: {
      ...EMPTY,
      price_velocity: hit.priceChangeM5Pct / 100,
      volume_short: hit.volumeM5Usd,
    },
    explanation: `[pump.fun] ${hit.symbol} early momentum · ${hit.buysM5} buys/5m · vol $${hit.volumeM5Usd.toFixed(0)} · ${hit.priceChangeM5Pct.toFixed(1)}% m5 · ${hit.graduationPct.toFixed(0)}% to grad`,
  };
}

export function scannerHitsToSignals(input: {
  solscan: ShitcoinWhaleHit[];
  pumpFun: PumpMomentumHit[];
}): SignalEvent[] {
  return [
    ...input.solscan.slice(0, 15).map(solscanHitToSignal),
    ...input.pumpFun.slice(0, 15).map(pumpHitToSignal),
  ].sort((a, b) => b.timestamp_micros - a.timestamp_micros);
}
