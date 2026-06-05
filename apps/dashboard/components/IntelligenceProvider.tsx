'use client';

import { createContext, useContext, useEffect, type ReactNode } from 'react';

import { intelligenceStore } from '@/lib/intelligence/store';

const IntelContext = createContext(false);

export function useIntelContext() {
  return useContext(IntelContext);
}

export function IntelligenceProvider({
  url,
  children,
}: {
  url: string;
  children: ReactNode;
}) {
  useEffect(() => {
    let ws: WebSocket | null = null;
    let timer: ReturnType<typeof setTimeout> | null = null;
    let closed = false;

    const connect = () => {
      if (closed) return;
      ws = new WebSocket(url);

      ws.onopen = () => intelligenceStore.setConnected(true);

      ws.onmessage = (ev) => {
        if (typeof ev.data === 'string') intelligenceStore.ingestRaw(ev.data);
      };

      ws.onclose = () => {
        intelligenceStore.setConnected(false);
        if (!closed) timer = setTimeout(connect, 3000);
      };

      ws.onerror = () => ws?.close();
    };

    connect();

    return () => {
      closed = true;
      if (timer) clearTimeout(timer);
      ws?.close();
      intelligenceStore.setConnected(false);
    };
  }, [url]);

  return <IntelContext.Provider value>{children}</IntelContext.Provider>;
}
