import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { streamStore } from '@/lib/stream-store';
import type { SignalEvent, TradeEvent } from '@/lib/types';

function makeTrade(id: string, stage: TradeEvent['stage'], ts: number): TradeEvent {
  return {
    v: 1,
    trade_id: id,
    wallet_id: 'paper',
    source_strategy: 'scalp',
    pair: 'SOL/USDC',
    side: 'long',
    size_usd: 200,
    expected_pnl_usd: 1.5,
    tx_signature: null,
    timestamp_us: ts,
    stage,
    mode: 'paper',
    signal_id: 42,
  };
}

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

describe('streamStore trade ingestion', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('tracks latest stage per trade_id', () => {
    streamStore.ingestRaw(
      JSON.stringify({
        type: 'batch',
        seq: 1,
        ts_micros: 1,
        events: [
          { type: 'trade', data: makeTrade('t1', 'started', 100) },
          { type: 'trade', data: makeTrade('t1', 'filled', 200) },
        ],
      }),
    );
    flushStore();

    const trades = streamStore.getTradesSnapshot(10);
    expect(trades).toHaveLength(1);
    expect(trades[0].stage).toBe('filled');
    expect(streamStore.getLatestTrade()?.trade_id).toBe('t1');
  });

  it('replays journal events on reconnect batch', () => {
    streamStore.ingestRaw(
      JSON.stringify({
        type: 'batch',
        seq: 0,
        ts_micros: 0,
        events: [
          { type: 'health', data: { status: 'connected', uptime_secs: 1, signals_stored: 0, version: '0.1' } },
          { type: 'trade', data: makeTrade('replay-1', 'quoted', 50) },
          { type: 'trade', data: makeTrade('replay-2', 'filled', 60) },
        ],
      }),
    );
    flushStore();

    const trades = streamStore.getTradesSnapshot(10);
    const ids = trades.map((t) => t.trade_id);
    expect(ids).toContain('replay-1');
    expect(ids).toContain('replay-2');
  });
});
