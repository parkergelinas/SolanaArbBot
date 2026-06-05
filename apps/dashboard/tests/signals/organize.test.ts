import { describe, expect, it } from 'vitest';

import {
  filterSignals,
  mergeSignals,
  sortSignals,
} from '@/lib/signals';
import type { SignalEvent } from '@/lib/types';

function makeSignal(id: number, overrides: Partial<SignalEvent> = {}): SignalEvent {
  return {
    signal_id: id,
    timestamp_micros: id * 1_000_000,
    pool_address: `pool_${id}`,
    signal_type: 'Momentum',
    strength: 0.5,
    confidence: 0.6,
    direction: 'Long',
    timeframe_secs: 60,
    feature_vector: {
      volume_short: 1,
      volume_long: 1,
      price_velocity: 0,
      liquidity_delta_pct: 0,
      whale_activity_score: 0,
      smart_money_score: 0,
      data_points: 10,
    },
    explanation: 'test',
    ...overrides,
  };
}

describe('signal organization', () => {
  it('merges live and historical without duplicates', () => {
    const historical = [makeSignal(1), makeSignal(2)];
    const live = [makeSignal(2, { strength: 0.9 }), makeSignal(3)];
    const { signals, liveIds } = mergeSignals(live, historical);
    expect(signals).toHaveLength(3);
    expect(signals[0].signal_id).toBe(3);
    expect(signals.find((s) => s.signal_id === 2)?.strength).toBe(0.9);
    expect(liveIds.has(3)).toBe(true);
  });

  it('filters by type and direction', () => {
    const signals = [
      makeSignal(1, { signal_type: 'WhaleFlow', direction: 'Long' }),
      makeSignal(2, { signal_type: 'Momentum', direction: 'Short' }),
    ];
    expect(filterSignals(signals, 'WhaleFlow', 'All')).toHaveLength(1);
    expect(filterSignals(signals, 'All', 'Short')).toHaveLength(1);
  });

  it('sorts by strength descending', () => {
    const signals = [
      makeSignal(1, { strength: 0.2 }),
      makeSignal(2, { strength: 0.9 }),
    ];
    const sorted = sortSignals(signals, 'strength');
    expect(sorted[0].signal_id).toBe(2);
  });
});
