'use client';

import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { streamStore } from './stream-store';
import type { WsEvent } from './types';

// ─── useInterval ─────────────────────────────────────────────────────────────

export function useInterval(callback: () => void, delay: number | null) {
  const cb = useRef(callback);
  useEffect(() => { cb.current = callback; }, [callback]);
  useEffect(() => {
    if (delay === null) return;
    const id = setInterval(() => cb.current(), delay);
    return () => clearInterval(id);
  }, [delay]);
}

// ─── Stream hooks (batched external store) ───────────────────────────────────

export function useStreamSignals<T = import('./types').SignalEvent>(
  maxItems = 200,
): T[] {
  return useSyncExternalStore(
    (onStoreChange) => streamStore.subscribe(onStoreChange),
    () => streamStore.getSignalsSnapshot(maxItems) as T[],
    () => [] as T[],
  );
}

export function useStreamTrades<T = import('./types').TradeEvent>(
  maxItems = 200,
): T[] {
  return useSyncExternalStore(
    (onStoreChange) => streamStore.subscribe(onStoreChange),
    () => streamStore.getTradesSnapshot(maxItems) as T[],
    () => [] as T[],
  );
}

export function useLatestTrade<T = import('./types').TradeEvent>(): T | null {
  return useSyncExternalStore(
    (onStoreChange) => streamStore.subscribe(onStoreChange),
    () => streamStore.getLatestTrade() as T | null,
    () => null,
  );
}

export function useStreamLatest<T>(type: WsEvent['type']): T | null {
  return useSyncExternalStore(
    (onStoreChange) => streamStore.subscribe(onStoreChange),
    () => streamStore.getLatest<T>(type),
    () => null,
  );
}

/** @deprecated Use useStreamSignals — kept for gradual migration */
export function useWsEvents<T = unknown>(
  type: WsEvent['type'],
  maxItems = 200,
): T[] {
  if (type === 'signal') {
    return useStreamSignals<T>(maxItems);
  }
  const latest = useStreamLatest<T>(type);
  return latest ? [latest] : [];
}

/** @deprecated Use useStreamLatest */
export function useLatestWsEvent<T = unknown>(type: WsEvent['type']): T | null {
  return useStreamLatest<T>(type);
}

// ─── useFetch (REST bootstrap only — pass null interval to disable polling) ─

export function useFetch<T>(
  fetcher: () => Promise<T>,
  intervalMs: number | null = null,
): { data: T | null; error: string | null; loading: boolean; refetch: () => void } {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refetch = useCallback(async () => {
    try {
      setError(null);
      const d = await fetcher();
      setData(d);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [fetcher]);

  useEffect(() => { refetch(); }, [refetch]);
  useInterval(refetch, intervalMs);

  return { data, error, loading, refetch };
}
