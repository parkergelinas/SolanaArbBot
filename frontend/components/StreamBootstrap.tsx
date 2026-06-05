'use client';

import { useEffect } from 'react';
import { intelligenceStore } from '@/lib/stream-store';

const URL =
  process.env.NEXT_PUBLIC_INTELLIGENCE_URL ?? 'ws://localhost:8090/intelligence';

export default function StreamBootstrap() {
  useEffect(() => {
    let ws: WebSocket | null = null;
    let retry: ReturnType<typeof setTimeout> | null = null;

    const connect = () => {
      ws = new WebSocket(URL);
      ws.onopen = () => intelligenceStore.setConnected(true);
      ws.onmessage = (ev) => intelligenceStore.ingestRaw(ev.data as string);
      ws.onclose = () => {
        intelligenceStore.setConnected(false);
        retry = setTimeout(connect, 3000);
      };
      ws.onerror = () => ws?.close();
    };

    connect();
    return () => {
      if (retry) clearTimeout(retry);
      ws?.close();
    };
  }, []);

  return null;
}
