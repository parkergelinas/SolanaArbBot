'use client';

import { useEffect } from 'react';

import { useWatchlistStore } from '@/stores/watchlistStore';

export default function WatchlistBootstrap() {
  const hydrate = useWatchlistStore((s) => s.hydrate);
  const hydrated = useWatchlistStore((s) => s.hydrated);

  useEffect(() => {
    if (!hydrated) hydrate();
  }, [hydrate, hydrated]);

  return null;
}
