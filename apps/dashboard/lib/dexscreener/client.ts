import { SOL_MINT, USDC_MINT } from '@/lib/terminal/tokens';

import type { DexPair, DexPairSnapshot, DexTokensResponse } from './types';

const API_BASE =
  typeof process !== 'undefined' && process.env.NEXT_PUBLIC_DEXSCREENER_API
    ? process.env.NEXT_PUBLIC_DEXSCREENER_API.replace(/\/$/, '')
    : 'https://api.dexscreener.com/latest/dex';

const TOKEN_PAIRS_V1 = 'https://api.dexscreener.com/token-pairs/v1/solana';

export function dexScreenerEmbedUrl(pairAddress: string): string {
  const params = new URLSearchParams({
    embed: '1',
    theme: 'dark',
    chartTheme: 'dark',
    chartStyle: '1',
    chartType: 'price',
    interval: '15',
    info: '0',
    trades: '0',
    loadChartSettings: '0',
  });
  return `https://dexscreener.com/solana/${pairAddress}?${params.toString()}`;
}

export function dexScreenerPairPageUrl(pairAddress: string): string {
  return `https://dexscreener.com/solana/${pairAddress}`;
}

function num(v: string | number | undefined): number {
  if (typeof v === 'number') return v;
  if (typeof v === 'string') {
    const n = parseFloat(v);
    return Number.isFinite(n) ? n : 0;
  }
  return 0;
}

/** Prefer highest-liquidity Solana pair that includes `mint`. */
export function pickBestPair(pairs: DexPair[] | null | undefined, mint: string): DexPair | null {
  if (!pairs?.length) return null;

  const relevant = pairs.filter(
    (p) =>
      p.chainId === 'solana' &&
      (p.baseToken?.address === mint || p.quoteToken?.address === mint),
  );

  if (relevant.length === 0) return null;

  return [...relevant].sort(
    (a, b) => (b.liquidity?.usd ?? 0) - (a.liquidity?.usd ?? 0),
  )[0];
}

export function pairToSnapshot(pair: DexPair): DexPairSnapshot {
  return {
    pairAddress: pair.pairAddress,
    dexId: pair.dexId,
    pairUrl: pair.url || dexScreenerPairPageUrl(pair.pairAddress),
    baseSymbol: pair.baseToken?.symbol ?? '?',
    quoteSymbol: pair.quoteToken?.symbol ?? '?',
    priceUsd: num(pair.priceUsd),
    changeH24Pct: pair.priceChange?.h24 ?? 0,
    volumeH24Usd: pair.volume?.h24 ?? 0,
    liquidityUsd: pair.liquidity?.usd ?? 0,
  };
}

async function fetchJson<T>(url: string, signal?: AbortSignal): Promise<T> {
  const res = await fetch(url, { signal, cache: 'no-store' });
  if (!res.ok) throw new Error(`DexScreener ${res.status}`);
  return res.json() as Promise<T>;
}

async function fetchPairsForMint(mint: string, signal?: AbortSignal): Promise<DexPair[]> {
  // v1 endpoint returns a flat array — best for single-token lookup.
  try {
    const v1 = await fetchJson<DexPair[]>(`${TOKEN_PAIRS_V1}/${mint}`, signal);
    if (Array.isArray(v1) && v1.length > 0) return v1;
  } catch {
    // fall through to legacy endpoint
  }

  const legacy = await fetchJson<DexTokensResponse>(`${API_BASE}/tokens/${mint}`, signal);
  return legacy.pairs ?? [];
}

/**
 * Resolve the top liquidity pair for a mint via DexScreener public API.
 * For native SOL, also tries USDC-quoted pairs to avoid truncated global SOL lists.
 */
export async function resolvePairForMint(
  mint: string,
  signal?: AbortSignal,
): Promise<DexPairSnapshot | null> {
  let pairs = await fetchPairsForMint(mint, signal);
  let best = pickBestPair(pairs, mint);

  if (!best && mint === SOL_MINT) {
    pairs = await fetchPairsForMint(USDC_MINT, signal);
    best =
      pickBestPair(pairs, SOL_MINT) ??
      pairs
        .filter(
          (p) =>
            p.chainId === 'solana' &&
            (p.baseToken?.symbol === 'SOL' || p.quoteToken?.symbol === 'SOL'),
        )
        .sort((a, b) => (b.liquidity?.usd ?? 0) - (a.liquidity?.usd ?? 0))[0] ??
      null;
  }

  return best ? pairToSnapshot(best) : null;
}
