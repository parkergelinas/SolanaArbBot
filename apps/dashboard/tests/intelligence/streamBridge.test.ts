import { describe, expect, it } from 'vitest';

import { streamSignalToEvent, streamSignalsToEvents } from '@/lib/intelligence/streamBridge';
import type { Signal } from '@/lib/stream/types';

function makeStreamSignal(overrides: Partial<Signal> = {}): Signal {
  return {
    v: 1,
    signal_id: 'sig_whale_1',
    mint: 'So11111111111111111111111111111111111111112',
    kind: 'whale_flow',
    strength: 0.82,
    confidence: 0.71,
    timestamp_ms: 1_700_000_000_000,
    detail: 'large buy detected',
    ...overrides,
  };
}

describe('streamBridge', () => {
  it('maps whale_flow to WhaleFlow signal event', () => {
    const ev = streamSignalToEvent(makeStreamSignal());
    expect(ev.signal_type).toBe('WhaleFlow');
    expect(ev.direction).toBe('Long');
    expect(ev.explanation).toContain('[stream-api]');
    expect(ev.explanation).toContain('large buy detected');
    expect(ev.signal_id).toBeLessThan(0);
  });

  it('maps smart_money kind correctly', () => {
    const ev = streamSignalToEvent(makeStreamSignal({ kind: 'smart_money', signal_id: 'sig_smart_2' }));
    expect(ev.signal_type).toBe('SmartMoney');
    expect(ev.feature_vector.smart_money_score).toBe(0.71);
  });

  it('sorts events newest first', () => {
    const events = streamSignalsToEvents([
      makeStreamSignal({ signal_id: 'a', timestamp_ms: 100 }),
      makeStreamSignal({ signal_id: 'b', timestamp_ms: 200 }),
    ]);
    expect(events[0].timestamp_micros).toBeGreaterThan(events[1].timestamp_micros);
  });
});
