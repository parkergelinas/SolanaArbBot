import type { SignalEvent } from '@/lib/types';
import type { ArbOpportunity } from '@/stores/marketStore';

function arbSignalId(id: string): number {
  let h = 0;
  for (let i = 0; i < id.length; i++) {
    h = (h * 31 + id.charCodeAt(i)) | 0;
  }
  return h > 0 ? -h - 2_000_000 : h - 2_000_000;
}

export function arbOpportunityToSignal(opp: ArbOpportunity): SignalEvent {
  const profit = opp.estimatedProfitUsd ?? 0;
  const winProb = opp.winProbability ?? 0.5;
  const strength = Math.min(1, opp.spreadBps / 100);
  const confidence = Math.min(1, winProb);

  const sym = opp.symbol ?? opp.token.slice(0, 8);
  const buyLabel = opp.buyDexLabel ?? opp.buyDex;
  const sellLabel = opp.sellDexLabel ?? opp.sellDex;
  const source = opp.source ?? 'stream';

  return {
    signal_id: arbSignalId(opp.id),
    timestamp_micros: opp.timestamp_ms * 1000,
    pool_address: opp.token,
    signal_type: 'Arb',
    strength,
    confidence,
    direction: 'Long',
    timeframe_secs: 60,
    feature_vector: {
      volume_short: profit,
      volume_long: opp.minLiquidityUsd ?? 0,
      price_velocity: opp.spreadBps / 10_000,
      liquidity_delta_pct: 0,
      whale_activity_score: 0,
      smart_money_score: 0,
      data_points: opp.venueCount ?? 2,
    },
    explanation: `[${source}] ${sym} cross-DEX +${opp.spreadBps.toFixed(1)} bps · buy ${buyLabel} @ $${opp.buyPrice.toFixed(4)} → sell ${sellLabel} @ $${opp.sellPrice.toFixed(4)} · est $${profit.toFixed(2)} · ${(winProb * 100).toFixed(0)}% fill prob`,
  };
}

export function arbOpportunitiesToSignals(opps: ArbOpportunity[]): SignalEvent[] {
  return opps.map(arbOpportunityToSignal).sort((a, b) => b.timestamp_micros - a.timestamp_micros);
}
