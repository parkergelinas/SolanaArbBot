import { fetchTokenPairs, type DexPair } from '@/lib/scanners/dexscreener';
import type { Dex } from '@/lib/stream/types';
import type { ArbOpportunity } from '@/stores/marketStore';

const DEFAULT_TRADE_USD = 250;
const MIN_SPREAD_BPS = 8;
const MIN_LIQ_USD = 15_000;

function normalizeDex(dexId: string | undefined): Dex | null {
  const d = (dexId ?? '').toLowerCase();
  if (d.includes('raydium')) return 'raydium';
  if (d.includes('orca')) return 'orca';
  if (d.includes('jupiter') || d.includes('metis')) return 'jupiter';
  if (d.includes('pump')) return 'pump';
  return null;
}

function parsePrice(pair: DexPair): number | null {
  const p = Number(pair.priceUsd);
  return Number.isFinite(p) && p > 0 ? p : null;
}

function estimateWinProbability(spreadBps: number, minLiqUsd: number): number {
  const spreadScore = Math.min(1, spreadBps / 80);
  const liqScore = Math.min(1, minLiqUsd / 200_000);
  return Math.min(0.95, 0.35 + spreadScore * 0.35 + liqScore * 0.25);
}

function pairToOpportunity(
  mint: string,
  symbol: string,
  entries: { dex: Dex; price: number; liq: number; dexLabel: string }[],
  tradeUsd: number,
  ts: number,
): ArbOpportunity | null {
  if (entries.length < 2) return null;

  let bestBuy = entries[0];
  let bestSell = entries[0];
  for (const e of entries) {
    if (e.price < bestBuy.price) bestBuy = e;
    if (e.price > bestSell.price) bestSell = e;
  }
  if (bestSell.price <= bestBuy.price) return null;

  const spreadBps = ((bestSell.price - bestBuy.price) / bestBuy.price) * 10_000;
  if (spreadBps < MIN_SPREAD_BPS) return null;

  const minLiq = Math.min(bestBuy.liq, bestSell.liq);
  const grossUsd = tradeUsd * (spreadBps / 10_000);
  const feesUsd = tradeUsd * 0.002 + 0.08;
  const estimatedProfitUsd = Math.max(0, grossUsd - feesUsd);

  return {
    id: `ds-${mint}-${bestBuy.dex}-${bestSell.dex}-${ts}`,
    token: mint,
    symbol,
    buyDex: bestBuy.dex,
    sellDex: bestSell.dex,
    buyPrice: bestBuy.price,
    sellPrice: bestSell.price,
    spreadBps,
    timestamp_ms: ts,
    source: 'dexscreener',
    estimatedProfitUsd,
    minLiquidityUsd: minLiq,
    venueCount: entries.length,
    winProbability: estimateWinProbability(spreadBps, minLiq),
    buyDexLabel: bestBuy.dexLabel,
    sellDexLabel: bestSell.dexLabel,
  };
}

/** Scan DexScreener pairs for cross-venue spreads on curated mints. */
export async function scanCrossDexArb(
  mints: { mint: string; symbol: string }[],
  tradeUsd = DEFAULT_TRADE_USD,
): Promise<ArbOpportunity[]> {
  const ts = Date.now();
  const out: ArbOpportunity[] = [];

  await Promise.all(
    mints.map(async ({ mint, symbol }) => {
      const pairs = await fetchTokenPairs(mint);
      const byDex = new Map<Dex, { dex: Dex; price: number; liq: number; dexLabel: string }>();

      for (const pair of pairs) {
        const dex = normalizeDex(pair.dexId);
        const price = parsePrice(pair);
        const liq = pair.liquidity?.usd ?? 0;
        if (!dex || !price || liq < MIN_LIQ_USD) continue;

        const prev = byDex.get(dex);
        if (!prev || liq > prev.liq) {
          byDex.set(dex, {
            dex,
            price,
            liq,
            dexLabel: pair.dexId ?? dex,
          });
        }
      }

      const opp = pairToOpportunity(mint, symbol, Array.from(byDex.values()), tradeUsd, ts);
      if (opp) out.push(opp);
    }),
  );

  return out.sort((a, b) => b.spreadBps - a.spreadBps);
}
