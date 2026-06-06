'use client';

import { useCallback, useEffect, useState } from 'react';

import { resolvePairForMint } from '@/lib/dexscreener/client';
import type { DexPairSnapshot } from '@/lib/dexscreener/types';
import { WATCHLIST } from '@/lib/terminal/tokens';
import { useFeedsStore } from '@/stores/feedsStore';

const POLL_MS = 45_000;

export type WatchlistDexMap = Record<string, DexPairSnapshot | null>;

/**
 * Poll DexScreener for all watchlist mints — 24h change + reference USD.
 */
export function useDexScreenerWatchlist(): {
  snapshots: WatchlistDexMap;
  refresh: () => void;
  loading: boolean;
} {
  const [snapshots, setSnapshots] = useState<WatchlistDexMap>({});
  const [loading, setLoading] = useState(true);
  const setDexScreener = useFeedsStore((s) => s.setDexScreener);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const results = await Promise.allSettled(
        WATCHLIST.map(async (t) => {
          const snap = await resolvePairForMint(t.mint);
          return [t.mint, snap] as const;
        }),
      );

      const next: WatchlistDexMap = {};
      let hits = 0;
      for (const r of results) {
        if (r.status === 'fulfilled') {
          const [mint, snap] = r.value;
          next[mint] = snap;
          if (snap) hits += 1;
        }
      }
      setSnapshots((prev) => ({ ...prev, ...next }));
      setDexScreener({
        status: hits > 0 ? 'online' : 'degraded',
        lastOkAt: Date.now(),
        detail: `${hits}/${WATCHLIST.length} pairs`,
      });
    } catch {
      setDexScreener({ status: 'offline', lastOkAt: Date.now() });
    } finally {
      setLoading(false);
    }
  }, [setDexScreener]);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, POLL_MS);
    return () => clearInterval(id);
  }, [refresh]);

  return { snapshots, refresh, loading };
}
