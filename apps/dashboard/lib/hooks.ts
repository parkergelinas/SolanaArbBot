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

export function useWsEvents<K extends WsEvent['type']>(
  type: K,
  maxItems = 200,
): Extract<WsEvent, { type: K }>['data'][] {
  type Data = Extract<WsEvent, { type: K }>['data'];
  const [items, setItems] = useState<Data[]>([]);

  // This hook reads from the global WS context which is set up in WebSocketProvider.
  // We expose a custom browser event for each WsEvent so any component can subscribe.
  const handleEvent = useCallback(
    (e: Event) => {
      const ev = (e as CustomEvent<WsEvent>).detail;
      if (ev.type !== type) return;
      const data = ev.data as Data;
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

export function useLatestWsEvent<K extends WsEvent['type']>(
  type: K,
): Extract<WsEvent, { type: K }>['data'] | null {
  const events = useWsEvents(type, 1);
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
