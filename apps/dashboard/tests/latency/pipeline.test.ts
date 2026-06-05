import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { StreamClient } from '@/lib/stream/client';
import { useMarketStore } from '@/stores/marketStore';
import { batchJson, makeBatch, makeSwap, swapMessage } from '../helpers/fixtures';

describe('latency pipeline simulation', () => {
  let client: StreamClient;
  let rafCb: FrameRequestCallback | null;

  beforeEach(() => {
    useMarketStore.getState().clear();
    client = new StreamClient();
    rafCb = null;
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      rafCb = cb;
      return 1;
    });
  });

  afterEach(() => {
    client.disconnect();
    vi.unstubAllGlobals();
  });

  it('ingest delay: messages buffer until rAF flush', async () => {
    const ingestMs: number[] = [];
    client.subscribe((_msgs, meta) => {
      ingestMs.push(meta.flushedAt - meta.receivedAt);
    });

    const frame = makeBatch([swapMessage(makeSwap())], 1, Date.now() - 30);
    client.onMessage(batchJson(frame));

    await new Promise((r) => setTimeout(r, 20));
    expect(ingestMs).toHaveLength(0);

    rafCb!(0);
    expect(ingestMs).toHaveLength(1);
    expect(ingestMs[0]).toBeGreaterThanOrEqual(0);
  });

  it('burst traffic: store apply stays under budget for 500-msg batch', () => {
    const messages = Array.from({ length: 500 }, (_, i) =>
      swapMessage(makeSwap({ signature: `burst_${i}` })),
    );

    const renderStart = performance.now();
    useMarketStore.getState().applyMessages(messages, { seq: 1, ts_ms: Date.now() });
    const renderMs = performance.now() - renderStart;

    expect(renderMs).toBeLessThan(150);
    expect(useMarketStore.getState().swaps.length).toBeLessThanOrEqual(500);
  });

  it('e2e simulated path: client flush → store apply', () => {
    const timings: { ingest: number; render: number }[] = [];

    client.subscribe((messages, meta) => {
      const renderStart = performance.now();
      useMarketStore.getState().applyMessages(messages, meta);
      const renderEnd = performance.now();
      timings.push({
        ingest: meta.flushedAt - meta.receivedAt,
        render: renderEnd - renderStart,
      });
    });

    for (let i = 0; i < 50; i++) {
      client.onMessage(
        batchJson(makeBatch([swapMessage(makeSwap({ signature: `e2e_${i}` }))], i)),
      );
      rafCb!(0);
      vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
        rafCb = cb;
        return 1;
      });
    }

    expect(timings).toHaveLength(50);
    for (const t of timings) {
      expect(t.render).toBeLessThan(150);
    }
    expect(useMarketStore.getState().swaps.length).toBe(50);
  });
});
