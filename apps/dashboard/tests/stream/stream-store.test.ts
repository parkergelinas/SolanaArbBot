import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { streamStore } from '@/lib/stream-store';
import type { SignalEvent } from '@/lib/types';

function makeSignal(id: number): SignalEvent {
  return {
    signal_id: id,
    timestamp_micros: id,
    pool_address: 'pool',
    signal_type: 'Momentum',
    strength: 0.5,
    confidence: 0.5,
    direction: 'Long',
    timeframe_secs: 60,
    feature_vector: {
      volume_short: 0,
      volume_long: 0,
      price_velocity: 0,
      liquidity_delta_pct: 0,
      whale_activity_score: 0,
      smart_money_score: 0,
      data_points: 1,
    },
    explanation: 'test',
  };
}

function flushStore() {
  vi.advanceTimersByTime(50);
}

describe('streamStore snapshot stability', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns the same array reference when store version is unchanged', () => {
    const a = streamStore.getSignalsSnapshot(100);
    const b = streamStore.getSignalsSnapshot(100);
    expect(a).toBe(b);
  });

  it('returns a new array reference after a new signal is ingested', () => {
    streamStore.ingestRaw(
      JSON.stringify({ type: 'signal', data: makeSignal(99_001) }),
    );
    flushStore();
    const before = streamStore.getSignalsSnapshot(100);

    streamStore.ingestRaw(
      JSON.stringify({ type: 'signal', data: makeSignal(99_002) }),
    );
    flushStore();
    const after = streamStore.getSignalsSnapshot(100);

    expect(after).not.toBe(before);
    expect(after.at(-1)?.signal_id).toBe(99_002);
  });
});
