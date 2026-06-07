/**
 * DexScreener price client — free API, no auth, generous rate limits.
 * Used as a fallback when Jupiter's price API is rate-limited.
 *
 * API quirk: multi-token queries return a flat DsPair[], single-token returns
 * { pairs: DsPair[] }. This client normalises both shapes.
 */

const DS_BASE = 'https://api.dexscreener.com';
const CHUNK_SIZE = 30; // DS allows up to 30 addresses per request
const TIMEOUT_MS = 6_000;

interface DsPair {
  priceUsd?: string;
  baseToken?: { address?: string; symbol?: string };
  liquidity?: { usd?: number };
}

type DsRaw = DsPair[] | { pairs?: DsPair[] };

function normalisePairs(raw: unknown): DsPair[] {
  if (Array.isArray(raw)) return raw as DsPair[];
  if (raw && typeof raw === 'object' && 'pairs' in raw) {
    const p = (raw as { pairs?: unknown }).pairs;
    return Array.isArray(p) ? (p as DsPair[]) : [];
  }
  return [];
}

/**
 * Fetch USD prices for Solana token mints from DexScreener.
 * Returns a partial map — mints with no liquid pool are omitted.
 */
export async function fetchDexScreenerPrices(
  mints: string[],
): Promise<Record<string, number>> {
  const deduped = [...new Set(mints)].filter(Boolean);
  if (deduped.length === 0) return {};

  const out: Record<string, number> = {};

  for (let i = 0; i < deduped.length; i += CHUNK_SIZE) {
    const chunk = deduped.slice(i, i + CHUNK_SIZE);
    try {
      const url = `${DS_BASE}/tokens/v1/solana/${chunk.join(',')}`;
      const resp = await fetch(url, {
        headers: { Accept: 'application/json' },
        signal: AbortSignal.timeout(TIMEOUT_MS),
      });
      if (!resp.ok) continue;

      const pairs = normalisePairs(await resp.json() as DsRaw);

      // For each mint, keep the price from the highest-liquidity pool
      const best = new Map<string, { price: number; liq: number }>();

      for (const pair of pairs) {
        const addr = pair.baseToken?.address;
        if (!addr) continue;
        const price = pair.priceUsd ? parseFloat(pair.priceUsd) : NaN;
        if (!isFinite(price) || price <= 0) continue;
        const liq = pair.liquidity?.usd ?? 0;
        const prev = best.get(addr);
        if (!prev || liq > prev.liq) best.set(addr, { price, liq });
      }

      for (const [addr, { price }] of best) {
        out[addr] = price;
      }
    } catch {
      // Partial failure is fine — we get what we get
    }
  }

  return out;
}
