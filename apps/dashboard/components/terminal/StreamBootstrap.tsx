'use client';

import { useEffect } from 'react';

import { useStreamStore } from '@/stores/streamStore';

/** Connects the terminal to stream-api; clears market state on reconnect. */
export default function StreamBootstrap() {
  const subscribe = useStreamStore((s) => s.subscribe);

  useEffect(() => subscribe(), [subscribe]);

  return null;
}
