import { resolvePairForMint } from '@/lib/dexscreener/client';
import type { DexAnchor } from './types';

export interface BootstrapResult {
  anchors: Record<string, DexAnchor>;
  hits: number;
  errors: string[];
}

/** Fetch live USD anchors for mints via DexScreener. */
export async function bootstrapWatchlistPrices(
  mints: string[],
  signal?: AbortSignal,
): Promise<BootstrapResult> {
  const anchors: Record<string, DexAnchor> = {};
  const errors: string[] = [];
  let hits = 0;
  const fetchedAt = Date.now();

  if (mints.length === 0) {
    return { anchors, hits, errors };
  }

  const results = await Promise.allSettled(
    mints.map(async (mint) => {
      const snap = await resolvePairForMint(mint, signal);
      return { mint, snap };
    }),
  );

  for (const r of results) {
    if (r.status === 'rejected') {
      errors.push(r.reason instanceof Error ? r.reason.message : String(r.reason));
      continue;
    }
    const { mint, snap } = r.value;
    if (!snap?.priceUsd || snap.priceUsd <= 0) {
      errors.push(`No DexScreener price for ${mint.slice(0, 8)}…`);
      continue;
    }
    anchors[mint] = {
      priceUsd: snap.priceUsd,
      changeH24Pct: snap.changeH24Pct,
      volumeH24Usd: snap.volumeH24Usd,
      liquidityUsd: snap.liquidityUsd,
      fetchedAt,
    };
    hits++;
  }

  return { anchors, hits, errors };
}
