'use client';

import { useEffect, useState } from 'react';

import { resolvePairForMint } from '@/lib/dexscreener/client';
import type { DexPairSnapshot } from '@/lib/dexscreener/types';

const CACHE_MS = 60_000;
const cache = new Map<string, { snapshot: DexPairSnapshot | null; at: number }>();

export interface UseDexScreenerPairResult {
  snapshot: DexPairSnapshot | null;
  loading: boolean;
  error: string | null;
}

export function useDexScreenerPair(mint: string | null): UseDexScreenerPairResult {
  const [snapshot, setSnapshot] = useState<DexPairSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!mint) {
      setSnapshot(null);
      setLoading(false);
      setError(null);
      return;
    }

    const cached = cache.get(mint);
    if (cached && Date.now() - cached.at < CACHE_MS) {
      setSnapshot(cached.snapshot);
      setLoading(false);
      setError(cached.snapshot ? null : 'No DexScreener pair found');
      return;
    }

    const ac = new AbortController();
    setLoading(true);
    setError(null);

    resolvePairForMint(mint, ac.signal)
      .then((result) => {
        cache.set(mint, { snapshot: result, at: Date.now() });
        setSnapshot(result);
        setError(result ? null : 'No DexScreener pair found for this mint');
      })
      .catch((e: unknown) => {
        if (ac.signal.aborted) return;
        const msg = e instanceof Error ? e.message : 'DexScreener lookup failed';
        setError(msg);
        setSnapshot(null);
      })
      .finally(() => {
        if (!ac.signal.aborted) setLoading(false);
      });

    return () => ac.abort();
  }, [mint]);

  return { snapshot, loading, error };
}
