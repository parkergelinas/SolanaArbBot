'use client';

import { useEffect, useRef } from 'react';

import { useMarketStore } from '@/stores/marketStore';
import { useStreamStore } from '@/stores/streamStore';

/** Mount once to connect the stream client without coupling to React render. */
export default function StreamBootstrap() {
  const connect = useStreamStore((s) => s.connect);
  const disconnect = useStreamStore((s) => s.disconnect);
  const connected = useStreamStore((s) => s.connected);
  const wasLive = useRef(false);

  useEffect(() => {
    connect();
    return () => disconnect();
  }, [connect, disconnect]);

  // Drop demo/sim state when live stream first connects.
  useEffect(() => {
    if (connected && !wasLive.current) {
      useMarketStore.getState().clear();
      wasLive.current = true;
    }
    if (!connected) wasLive.current = false;
  }, [connected]);

  return null;
}
