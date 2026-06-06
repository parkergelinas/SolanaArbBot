import { create } from 'zustand';

import { throttle } from '@/lib/perf/throttle';
import { streamClient } from '@/lib/stream/client';
import { useMarketStore } from './marketStore';

const DEFAULT_URL =
  typeof process !== 'undefined' && process.env.NEXT_PUBLIC_STREAM_URL
    ? process.env.NEXT_PUBLIC_STREAM_URL
    : 'ws://localhost:8080/stream';

const LATENCY_THROTTLE_MS = 250;
const STALE_MS = 12_000;
const DEGRADED_LATENCY_MS = 2_500;

export type ConnectionMode = 'live' | 'sim' | 'degraded';

export interface LatencySnapshot {
  wsMs: number;
  ingestMs: number;
  renderMs: number;
  e2eMs: number;
  serverTsMs: number;
  seq: number;
}

function deriveConnectionMode(
  connected: boolean,
  lastMessageAt: number,
  wsMs: number,
): ConnectionMode {
  if (!connected) return 'sim';
  const stale = lastMessageAt > 0 && Date.now() - lastMessageAt > STALE_MS;
  if (stale || wsMs > DEGRADED_LATENCY_MS) return 'degraded';
  return 'live';
}

interface StreamState {
  connected: boolean;
  connectionMode: ConnectionMode;
  url: string;
  lastSeq: number;
  lastMessageAt: number;
  batchesFlushed: number;
  messagesApplied: number;
  error: string | null;
  latency: LatencySnapshot;

  connect: (url?: string) => void;
  disconnect: () => void;
  subscribe: () => () => void;
  refreshConnectionMode: () => void;
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

export const useStreamStore = create<StreamState>((set, get) => {
  const publishLatency = throttle(
    (patch: {
      lastSeq: number;
      lastMessageAt: number;
      batchesFlushed: number;
      messagesApplied: number;
      latency: LatencySnapshot;
      connectionMode: ConnectionMode;
    }) => {
      set(patch);
    },
    LATENCY_THROTTLE_MS,
  );

  return {
    connected: false,
    connectionMode: 'sim' as ConnectionMode,
    url: DEFAULT_URL,
    lastSeq: 0,
    lastMessageAt: 0,
    batchesFlushed: 0,
    messagesApplied: 0,
    error: null,
    latency: emptyLatency(),

    refreshConnectionMode: () => {
      const { connected, lastMessageAt, latency } = get();
      set({ connectionMode: deriveConnectionMode(connected, lastMessageAt, latency.wsMs) });
    },

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

          const lastMessageAt = Date.now();
          const latency: LatencySnapshot = {
            wsMs,
            ingestMs,
            renderMs,
            e2eMs,
            serverTsMs: meta.ts_ms,
            seq: meta.seq,
          };
          const connectionMode = deriveConnectionMode(true, lastMessageAt, wsMs);

          set((s) => ({
            lastSeq: meta.seq,
            lastMessageAt,
            batchesFlushed: s.batchesFlushed + 1,
            messagesApplied: s.messagesApplied + messages.length,
            connectionMode,
          }));

          publishLatency({
            lastSeq: meta.seq,
            lastMessageAt,
            batchesFlushed: get().batchesFlushed,
            messagesApplied: get().messagesApplied,
            latency,
            connectionMode,
          });
        });
      }

      streamClient.disconnect();
      streamClient.connect(target, {
        onOpen: () =>
          set({
            connected: true,
            error: null,
            connectionMode: deriveConnectionMode(true, get().lastMessageAt, get().latency.wsMs),
          }),
        onClose: () => set({ connected: false, connectionMode: 'sim' }),
        onError: (err) => set({ error: String(err) }),
        onFrameReceived: ({ receivedAt }) => {
          lastFrameReceivedAt = receivedAt;
        },
      });
    },

    disconnect: () => {
      streamClient.disconnect();
      set({ connected: false, connectionMode: 'sim' });
    },

    subscribe: () => {
      get().connect();
      return () => get().disconnect();
    },
  };
});
