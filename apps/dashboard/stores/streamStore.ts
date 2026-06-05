import { create } from 'zustand';

import { streamClient } from '@/lib/stream/client';
import { useMarketStore } from './marketStore';

const DEFAULT_URL =
  typeof process !== 'undefined' && process.env.NEXT_PUBLIC_STREAM_URL
    ? process.env.NEXT_PUBLIC_STREAM_URL
    : 'ws://localhost:8080/stream';

export interface LatencySnapshot {
  wsMs: number;
  ingestMs: number;
  renderMs: number;
  e2eMs: number;
  serverTsMs: number;
  seq: number;
}

interface StreamState {
  connected: boolean;
  url: string;
  lastSeq: number;
  batchesFlushed: number;
  messagesApplied: number;
  error: string | null;
  latency: LatencySnapshot;

  connect: (url?: string) => void;
  disconnect: () => void;
  subscribe: () => () => void;
}

const emptyLatency = (): LatencySnapshot => ({
  wsMs: 0,
  ingestMs: 0,
  renderMs: 0,
  e2eMs: 0,
  serverTsMs: 0,
  seq: 0,
});

let unsubscribeFlush: (() => void) | null = null;
let lastFrameReceivedAt = 0;

export const useStreamStore = create<StreamState>((set, get) => ({
  connected: false,
  url: DEFAULT_URL,
  lastSeq: 0,
  batchesFlushed: 0,
  messagesApplied: 0,
  error: null,
  latency: emptyLatency(),

  connect: (url) => {
    const target = url ?? DEFAULT_URL;
    set({ url: target, error: null });

    if (!unsubscribeFlush) {
      unsubscribeFlush = streamClient.subscribe((messages, meta) => {
        const renderStart = performance.now();
        useMarketStore.getState().applyMessages(messages, meta);
        const renderEnd = performance.now();

        const now = Date.now();
        const wsMs = Math.max(0, now - meta.ts_ms);
        const ingestMs = Math.max(0, meta.flushedAt - meta.receivedAt);
        const renderMs = Math.max(0, renderEnd - renderStart);
        const e2eMs = Math.max(0, renderEnd - (lastFrameReceivedAt || meta.receivedAt));

        set((s) => ({
          lastSeq: meta.seq,
          batchesFlushed: s.batchesFlushed + 1,
          messagesApplied: s.messagesApplied + messages.length,
          latency: {
            wsMs,
            ingestMs,
            renderMs,
            e2eMs,
            serverTsMs: meta.ts_ms,
            seq: meta.seq,
          },
        }));
      });
    }

    streamClient.disconnect();
    streamClient.connect(target, {
      onOpen: () => set({ connected: true, error: null }),
      onClose: () => set({ connected: false }),
      onError: (err) => set({ error: String(err) }),
      onFrameReceived: ({ receivedAt }) => {
        lastFrameReceivedAt = receivedAt;
      },
    });
  },

  disconnect: () => {
    streamClient.disconnect();
    set({ connected: false });
  },

  subscribe: () => {
    get().connect();
    return () => get().disconnect();
  },
}));
