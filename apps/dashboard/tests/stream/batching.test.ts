import { describe, expect, it } from 'vitest';

import { isWSBatchFrame } from '@/lib/stream/types';
import { batchJson, makeBatch, makeSwap, swapMessage } from '../helpers/fixtures';

/** Simulates server-side 25–50ms batch window coalescing. */
function simulateServerBatcher(
  events: ReturnType<typeof swapMessage>[],
  windowMs = 33,
): ReturnType<typeof makeBatch>[] {
  const frames: ReturnType<typeof makeBatch>[] = [];
  let buffer: ReturnType<typeof swapMessage>[] = [];
  let seq = 0;
  let windowStart = 0;

  for (let i = 0; i < events.length; i++) {
    const t = i * 10;
    if (buffer.length === 0) windowStart = t;
    buffer.push(events[i]);
    const windowElapsed = t - windowStart;
    const isLast = i === events.length - 1;
    if (windowElapsed >= windowMs || isLast) {
      seq += 1;
      frames.push(makeBatch([...buffer], seq, t));
      buffer = [];
    }
  }
  return frames;
}

describe('batching protocol', () => {
  it('validates WSBatchFrame schema', () => {
    const frame = makeBatch([swapMessage(makeSwap())]);
    expect(isWSBatchFrame(frame)).toBe(true);
    expect(isWSBatchFrame({ type: 'batch', v: 2, messages: [] })).toBe(false);
  });

  it('serializes round-trip through JSON', () => {
    const raw = batchJson(makeBatch([swapMessage(makeSwap())], 42, 1_700_000_000_000));
    const parsed = JSON.parse(raw);
    expect(isWSBatchFrame(parsed)).toBe(true);
    expect(parsed.seq).toBe(42);
  });

  it('coalesces bursts into windows within 25–50ms policy', () => {
    const events = Array.from({ length: 20 }, (_, i) =>
      swapMessage(makeSwap({ signature: `s${i}` })),
    );
    const frames33 = simulateServerBatcher(events, 33);
    const frames25 = simulateServerBatcher(events, 25);
    expect(frames33.length).toBeLessThan(events.length);
    expect(frames25.length).toBeGreaterThanOrEqual(frames33.length);
    const total = frames33.flatMap((f) => f.messages).length;
    expect(total).toBe(events.length);
  });

  it('maintains monotonic seq across frames', () => {
    const events = Array.from({ length: 10 }, () => swapMessage(makeSwap()));
    const frames = simulateServerBatcher(events, 30);
    for (let i = 1; i < frames.length; i++) {
      expect(frames[i].seq).toBeGreaterThan(frames[i - 1].seq);
    }
  });
});
