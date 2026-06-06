const DEX_BASE = 'https://api.dexscreener.com';

export interface DexPair {
  chainId?: string;
  dexId?: string;
  pairAddress?: string;
  url?: string;
  pairCreatedAt?: number;
  baseToken?: { address?: string; symbol?: string; name?: string };
  quoteToken?: { address?: string; symbol?: string };
  priceUsd?: string;
  liquidity?: { usd?: number };
  marketCap?: number;
  fdv?: number;
  volume?: { m5?: number; h1?: number; h24?: number };
  txns?: { m5?: { buys?: number; sells?: number }; h1?: { buys?: number; sells?: number } };
  priceChange?: { m5?: number; h1?: number; h24?: number };
  labels?: string[];
}

export async function fetchDexPairs(url: string): Promise<DexPair[]> {
  const res = await fetch(url, {
    headers: { Accept: 'application/json' },
    next: { revalidate: 0 },
  });
  if (!res.ok) return [];
  const body = (await res.json()) as { pairs?: DexPair[] };
  return (body.pairs ?? []).filter((p) => p.chainId === 'solana');
}

export async function searchDexPairs(query: string): Promise<DexPair[]> {
  return fetchDexPairs(`${DEX_BASE}/latest/dex/search?q=${encodeURIComponent(query)}`);
}

export async function fetchTokenPairs(mint: string): Promise<DexPair[]> {
  return fetchDexPairs(`${DEX_BASE}/latest/dex/tokens/${mint}`);
}

export async function fetchLatestTokenProfiles(): Promise<
  { tokenAddress?: string; chainId?: string }[]
> {
  const res = await fetch(`${DEX_BASE}/token-profiles/latest/v1`, {
    headers: { Accept: 'application/json' },
    next: { revalidate: 0 },
  });
  if (!res.ok) return [];
  const body = (await res.json()) as { tokenAddress?: string; chainId?: string }[];
  return Array.isArray(body) ? body.filter((t) => t.chainId === 'solana') : [];
}

export function isPumpFunPair(pair: DexPair): boolean {
  const dex = (pair.dexId ?? '').toLowerCase();
  if (dex.includes('pump')) return true;
  return (pair.labels ?? []).some((l) => l.toLowerCase().includes('pump'));
}

export function pairAgeMinutes(pair: DexPair, nowMs = Date.now()): number {
  const created = pair.pairCreatedAt ?? 0;
  if (!created) return 9999;
  return Math.max(0, (nowMs - created) / 60_000);
}
