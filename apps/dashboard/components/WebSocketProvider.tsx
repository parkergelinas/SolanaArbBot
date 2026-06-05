'use client';

import React, { createContext, useContext, useEffect, useRef, useState } from 'react';
import type { WsEvent } from '@/lib/types';

interface WsContextValue {
  connected: boolean;
  lastEvent: WsEvent | null;
}

const WsContext = createContext<WsContextValue>({ connected: false, lastEvent: null });

export function useWsContext() {
  return useContext(WsContext);
}

export function WebSocketProvider({ url, children }: { url: string; children: React.ReactNode }) {
  const [connected, setConnected] = useState(false);
  const [lastEvent, setLastEvent] = useState<WsEvent | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const retryRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = () => {
    if (typeof window === 'undefined') return;

    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
    };

    ws.onmessage = (ev: MessageEvent) => {
      try {
        const event = JSON.parse(ev.data as string) as WsEvent;
        setLastEvent(event);
        // Dispatch as a browser custom event so any hook can subscribe without
        // prop-drilling.
        window.dispatchEvent(new CustomEvent('ws-event', { detail: event }));
      } catch {
        // ignore malformed frames
      }
    };

    ws.onclose = () => {
      setConnected(false);
      // Exponential back-off reconnect
      retryRef.current = setTimeout(connect, 3_000);
    };

    ws.onerror = () => {
      ws.close();
    };
  };

  useEffect(() => {
    connect();
    return () => {
      if (retryRef.current) clearTimeout(retryRef.current);
      wsRef.current?.close();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [url]);

  return (
    <WsContext.Provider value={{ connected, lastEvent }}>
      {children}
    </WsContext.Provider>
  );
}
