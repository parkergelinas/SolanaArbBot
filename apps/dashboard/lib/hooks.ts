'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
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

// ─── useWsEvents ──────────────────────────────────────────────────────────────
// Filtered view into the WebSocket stream; returns the last N matching events.

export function useWsEvents<T = unknown>(
  type: WsEvent['type'],
  maxItems = 200,
): T[] {
  const [items, setItems] = useState<T[]>([]);

  const handleEvent = useCallback(
    (e: Event) => {
      const ev = (e as CustomEvent<WsEvent>).detail;
      if (ev.type !== type) return;
      const data = ev.data as T;
      setItems(prev => {
        const next = [...prev, data];
        return next.length > maxItems ? next.slice(-maxItems) : next;
      });
    },
    [type, maxItems],
  );

  useEffect(() => {
    window.addEventListener('ws-event', handleEvent as EventListener);
    return () => window.removeEventListener('ws-event', handleEvent as EventListener);
  }, [handleEvent]);

  return items;
}

// ─── useLatestWsEvent ────────────────────────────────────────────────────────

export function useLatestWsEvent<T = unknown>(
  type: WsEvent['type'],
): T | null {
  const events = useWsEvents<T>(type, 1);
  return events[events.length - 1] ?? null;
}

// ─── useFetch ────────────────────────────────────────────────────────────────

export function useFetch<T>(
  fetcher: () => Promise<T>,
  intervalMs = 10_000,
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
