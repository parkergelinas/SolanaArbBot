'use client';

import React, { createContext, useContext, useEffect, useRef, useState } from 'react';
import { streamStore } from '@/lib/stream-store';

interface WsContextValue {
  connected: boolean;
}

const WsContext = createContext<WsContextValue>({ connected: false });

export function useWsContext() {
  return useContext(WsContext);
}

/**
 * Maintains a single WebSocket connection and feeds the external stream store.
 * React state updates only for connection status — not per market event.
 */
export function WebSocketProvider({ url, children }: { url: string; children: React.ReactNode }) {
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const retryRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (typeof window === 'undefined') return;

    const connect = () => {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => setConnected(true);

      ws.onmessage = (ev: MessageEvent) => {
        streamStore.ingestRaw(ev.data as string);
      };

      ws.onclose = () => {
        setConnected(false);
        retryRef.current = setTimeout(connect, 3_000);
      };

      ws.onerror = () => ws.close();
    };

    connect();

    return () => {
      if (retryRef.current) clearTimeout(retryRef.current);
      wsRef.current?.close();
    };
  }, [url]);

  return <WsContext.Provider value={{ connected }}>{children}</WsContext.Provider>;
}
