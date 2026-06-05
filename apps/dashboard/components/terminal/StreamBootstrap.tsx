'use client';

import { useEffect } from 'react';
import { useStreamStore } from '@/stores/streamStore';

/** Mount once to connect the stream client without coupling to React render. */
export default function StreamBootstrap() {
  const connect = useStreamStore((s) => s.connect);
  const disconnect = useStreamStore((s) => s.disconnect);

  useEffect(() => {
    connect();
    return () => disconnect();
  }, [connect, disconnect]);

  return null;
}
