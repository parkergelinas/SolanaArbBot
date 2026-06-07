import { beforeEach, describe, expect, it, vi } from 'vitest';

import { StreamClient } from '@/lib/stream/client';
import { useMarketStore } from '@/stores/marketStore';
import { batchJson, makeBatch, makeSwap, swapMessage } from '../helpers/fixtures';

describe('rendering stress', () => {
  beforeEach(() => {
    useMarketStore.getState().clear();
  });

  it('processes 10k swap events without unbounded growth', () => {
    const COUNT = 10_000;
    const messages = Array.from({ length: COUNT }, (_, i) =>
      swapMessage(makeSwap({ signature: `stress_${i}`, slot: 280_000_000 + i })),
    );

    const t0 = performance.now();
    useMarketStore.getState().applyMessages(messages, { seq: 1, ts_ms: Date.now() });
    const elapsed = performance.now() - t0;

    const state = useMarketStore.getState();
    expect(state.swaps).toHaveLength(500);
    expect(Object.keys(state.tokens).length).toBeLessThan(50);
    expect(elapsed).toBeLessThan(5_000);
  });

  it('rAF-batched client delivers all messages under burst load', () => {
    const client = new StreamClient();
    let rafCb: FrameRequestCallback | null = null;
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      rafCb = cb;
      return 1;
    });

    const received: number[] = [];
    client.subscribe((msgs) => {
      received.push(msgs.length);
    });

    const BATCHES = 200;
    let total = 0;
    for (let i = 0; i < BATCHES; i++) {
      const frame = makeBatch(
        [swapMessage(makeSwap({ signature: `b${i}` }))],
        i,
      );
      client.onMessage(batchJson(frame));
      total += 1;
    }

    expect(received).toHaveLength(0);
    while (rafCb) {
      const cb = rafCb as FrameRequestCallback;
      rafCb = null;
      cb(0);
      if (received.length < BATCHES) {
        vi.stubGlobal('requestAnimationFrame', (c: FrameRequestCallback) => {
          rafCb = c;
          return 1;
        });
      }
    }

    expect(received.reduce((a, b) => a + b, 0)).toBe(total);
    client.disconnect();
    vi.unstubAllGlobals();
  });

  it('continuous price ticks keep render apply under 150ms for 1k batch', () => {
    const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
    const messages = Array.from({ length: 1_000 }, (_, i) => ({
      type: 'token_price' as const,
      payload: {
        v: 1 as const,
        mint: USDC,
        price_usd: 1 + i * 0.0001,
        slot: i,
        timestamp_ms: Date.now(),
      },
    }));

    const t0 = performance.now();
    useMarketStore.getState().applyMessages(messages, { seq: 99, ts_ms: Date.now() });
    const renderMs = performance.now() - t0;

    expect(renderMs).toBeLessThan(150);
    expect(useMarketStore.getState().prices[USDC].price_usd).toBeCloseTo(1 + 999 * 0.0001, 4);
  });
});
