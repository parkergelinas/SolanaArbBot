import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { StreamClient } from '@/lib/stream/client';
import { batchJson, makeBatch, makeSwap, swapMessage } from '../helpers/fixtures';

describe('StreamClient', () => {
  let client: StreamClient;
  let rafCb: FrameRequestCallback | null;

  beforeEach(() => {
    client = new StreamClient();
    rafCb = null;
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      rafCb = cb;
      return 1;
    });
    vi.stubGlobal('cancelAnimationFrame', vi.fn());
  });

  afterEach(() => {
    client.disconnect();
    vi.unstubAllGlobals();
  });

  it('parses batch frames and rejects invalid JSON', () => {
    const handler = vi.fn();
    client.subscribe(handler);
    client.onMessage('not json');
    client.onMessage('{}');
    expect(handler).not.toHaveBeenCalled();
  });

  it('does not invoke subscribers synchronously from onMessage', () => {
    const handler = vi.fn();
    client.subscribe(handler);
    const frame = makeBatch([swapMessage(makeSwap())], 1);
    client.onMessage(batchJson(frame));
    expect(handler).not.toHaveBeenCalled();
    expect(rafCb).not.toBeNull();
    rafCb!(0);
    expect(handler).toHaveBeenCalledOnce();
  });

  it('coalesces multiple batches into one rAF flush', () => {
    const handler = vi.fn();
    client.subscribe(handler);
    client.onMessage(batchJson(makeBatch([swapMessage(makeSwap())], 1)));
    client.onMessage(batchJson(makeBatch([swapMessage(makeSwap())], 2)));
    rafCb!(0);
    expect(handler).toHaveBeenCalledOnce();
    const [, meta] = handler.mock.calls[0];
    expect(meta.messageCount).toBe(2);
  });

  it('preserves message ordering within a flush', () => {
    const handler = vi.fn();
    client.subscribe(handler);
    const a = makeSwap({ signature: 'first' });
    const b = makeSwap({ signature: 'second' });
    client.onMessage(batchJson(makeBatch([swapMessage(a)], 1)));
    client.onMessage(batchJson(makeBatch([swapMessage(b)], 2)));
    rafCb!(0);
    const [messages] = handler.mock.calls[0];
    expect(messages[0].type).toBe('swap');
    if (messages[0].type === 'swap') {
      expect(messages[0].payload.signature).toBe('first');
    }
    if (messages[1].type === 'swap') {
      expect(messages[1].payload.signature).toBe('second');
    }
  });

  it('includes latency metadata in flush meta', () => {
    const handler = vi.fn();
    client.subscribe(handler);
    const ts = Date.now() - 50;
    client.onMessage(batchJson(makeBatch([swapMessage(makeSwap())], 7, ts)));
    rafCb!(0);
    const [, meta] = handler.mock.calls[0];
    expect(meta.seq).toBe(7);
    expect(meta.ts_ms).toBe(ts);
    expect(meta.messageCount).toBe(1);
    expect(meta.flushedAt).toBeGreaterThanOrEqual(meta.receivedAt);
  });
});
