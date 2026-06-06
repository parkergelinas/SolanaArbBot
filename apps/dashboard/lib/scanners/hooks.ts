'use client';

import { useCallback, useEffect, useState } from 'react';

import { streamHttpBase } from '@/lib/config/streamHttp';

import type { PumpScannerResponse, SolscanResearcherResponse } from './types';

async function fetchJson<T>(path: string): Promise<T> {
  const res = await fetch(path, { cache: 'no-store' });
  if (!res.ok) throw new Error(`${path} → ${res.status}`);
  return res.json() as Promise<T>;
}

/** Prefer dashboard Next routes, then stream-api HTTP mirror. */
async function fetchScanner<T>(localPath: string, remotePath: string): Promise<T> {
  try {
    return await fetchJson<T>(localPath);
  } catch {
    const base = streamHttpBase();
    return fetchJson<T>(`${base}${remotePath}`);
  }
}

export function useSolscanResearcher(pollMs = 20_000) {
  const [data, setData] = useState<SolscanResearcherResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await fetchScanner<SolscanResearcherResponse>(
        '/api/scanners/solscan',
        '/api/scanners/solscan',
      );
      setData(next);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'fetch failed');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const id = setInterval(() => void refresh(), pollMs);
    return () => clearInterval(id);
  }, [refresh, pollMs]);

  return { data, loading, error, refresh };
}

export function usePumpFunScanner(pollMs = 15_000) {
  const [data, setData] = useState<PumpScannerResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await fetchScanner<PumpScannerResponse>(
        '/api/scanners/pump-fun',
        '/api/scanners/pump-fun',
      );
      setData(next);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'fetch failed');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const id = setInterval(() => void refresh(), pollMs);
    return () => clearInterval(id);
  }, [refresh, pollMs]);

  return { data, loading, error, refresh };
}
