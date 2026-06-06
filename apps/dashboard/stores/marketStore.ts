import { create } from 'zustand';

import {
  normalizeCandle,
  refPriceForMint,
  sanitizeTradePrice,
} from '@/lib/terminal/candles';
import type {
  Candle,
  CandleInterval,
  Dex,
  Signal,
  SwapEvent,
  TokenPrice,
  WSMessage,
} from '@/lib/stream/types';

export const MAX_SWAPS = 500;
const MAX_SIGNALS = 80;
const MAX_CANDLE_HISTORY = 120;

/** Chart is DexScreener embed — skip candle history to save memory. */
const STORE_CANDLES = false;

export function candleKey(mint: string, interval: CandleInterval) {
  return `${mint}:${interval}`;
}

export function shortMint(mint: string, head = 4, tail = 4) {
  if (mint.length <= head + tail + 1) return mint;
  return `${mint.slice(0, head)}…${mint.slice(-tail)}`;
}

export interface TokenRow {
  mint: string;
  price_usd: number;
  volume: number;
  buyFlow: number;
  sellFlow: number;
  flowImbalance: number;
  changePct: number;
  slot: number;
  timestamp_ms: number;
}

export type ArbSource = 'stream' | 'dexscreener' | 'jupiter';

export interface ArbOpportunity {
  id: string;
  token: string;
  symbol?: string;
  buyDex: Dex;
  sellDex: Dex;
  buyPrice: number;
  sellPrice: number;
  spreadBps: number;
  timestamp_ms: number;
  source?: ArbSource;
  estimatedProfitUsd?: number;
  minLiquidityUsd?: number;
  venueCount?: number;
  winProbability?: number;
  buyDexLabel?: string;
  sellDexLabel?: string;
}

export interface MarketState {
  prices: Record<string, TokenPrice>;
  tokens: Record<string, TokenRow>;
  swaps: SwapEvent[];
  candles: Record<string, Candle>;
  candleHistory: Record<string, Candle[]>;
  signals: Signal[];
  arbOpportunities: ArbOpportunity[];
  dexPrices: Record<string, Record<Dex, number>>;
  lastSeq: number;
  lastTsMs: number;

  applyMessages: (messages: WSMessage[], meta: { seq: number; ts_ms: number }) => void;
  mergeArbOpportunities: (external: ArbOpportunity[]) => void;
  clear: () => void;
}

const empty = (): Omit<MarketState, 'applyMessages' | 'mergeArbOpportunities' | 'clear'> => ({
  prices: {},
  tokens: {},
  swaps: [],
  candles: {},
  candleHistory: {},
  signals: [],
  arbOpportunities: [],
  dexPrices: {},
  lastSeq: 0,
  lastTsMs: 0,
});

function tokenDecimals(mint: string): number {
  if (mint.startsWith('So1111')) return 9;
  return 6;
}

export function amountToHuman(mint: string, raw: string): number {
  const n = Number(raw);
  if (!Number.isFinite(n)) return 0;
  return n / 10 ** tokenDecimals(mint);
}

/** USD price per unit of `tokenOut` from a swap leg. */
function swapPriceUsd(
  tokenIn: string,
  tokenOut: string,
  rawIn: string,
  rawOut: string,
  prices: Record<string, TokenPrice>,
): number {
  const inHuman = amountToHuman(tokenIn, rawIn);
  const outHuman = amountToHuman(tokenOut, rawOut);
  if (outHuman <= 0) return refPriceForMint(tokenOut, prices[tokenOut]?.price_usd);
  const usdIn = inHuman * refPriceForMint(tokenIn, prices[tokenIn]?.price_usd);
  const implied = usdIn / outHuman;
  return sanitizeTradePrice(tokenOut, implied, prices[tokenOut]?.price_usd);
}

function pushCandleHistory(
  history: Record<string, Candle[]>,
  key: string,
  candle: Candle,
): Record<string, Candle[]> {
  const prev = history[key] ?? [];
  const last = prev[prev.length - 1];
  let next: Candle[];
  if (last && last.ts_open_ms === candle.ts_open_ms) {
    next = [...prev.slice(0, -1), candle];
  } else if (!last || candle.ts_open_ms >= last.ts_open_ms) {
    next = [...prev, candle];
  } else {
    const idx = prev.findIndex((c) => c.ts_open_ms > candle.ts_open_ms);
    next =
      idx === -1
        ? [...prev, candle]
        : [...prev.slice(0, idx), candle, ...prev.slice(idx)];
  }
  if (next.length > MAX_CANDLE_HISTORY) {
    next = next.slice(-MAX_CANDLE_HISTORY);
  }
  return { ...history, [key]: next };
}

function recomputeArb(
  dexPrices: Record<string, Record<Dex, number>>,
  token: string,
  ts: number,
): ArbOpportunity[] {
  const byDex = dexPrices[token];
  if (!byDex) return [];
  const entries = Object.entries(byDex) as [Dex, number][];
  if (entries.length < 2) return [];

  let bestBuy: [Dex, number] | null = null;
  let bestSell: [Dex, number] | null = null;
  for (const [dex, price] of entries) {
    if (!bestBuy || price < bestBuy[1]) bestBuy = [dex, price];
    if (!bestSell || price > bestSell[1]) bestSell = [dex, price];
  }
  if (!bestBuy || !bestSell || bestSell[1] <= bestBuy[1]) return [];

  const spreadBps = ((bestSell[1] - bestBuy[1]) / bestBuy[1]) * 10_000;
  if (spreadBps < 5) return [];

  const grossUsd = 250 * (spreadBps / 10_000);
  return [
    {
      id: `${token}-${ts}`,
      token,
      buyDex: bestBuy[0],
      sellDex: bestSell[0],
      buyPrice: bestBuy[1],
      sellPrice: bestSell[1],
      spreadBps,
      timestamp_ms: ts,
      source: 'stream',
      estimatedProfitUsd: Math.max(0, grossUsd - 0.58),
      winProbability: Math.min(0.9, 0.4 + spreadBps / 120),
      venueCount: entries.length,
    },
  ];
}

function dedupeArb(opps: ArbOpportunity[]): ArbOpportunity[] {
  const byKey = new Map<string, ArbOpportunity>();
  for (const o of opps) {
    const key = `${o.token}:${o.buyDex}:${o.sellDex}`;
    const prev = byKey.get(key);
    if (!prev || o.spreadBps > prev.spreadBps) byKey.set(key, o);
  }
  return Array.from(byKey.values())
    .sort((a, b) => b.spreadBps - a.spreadBps)
    .slice(0, 40);
}

export const useMarketStore = create<MarketState>((set) => ({
  ...empty(),

  applyMessages: (messages, meta) => {
    set((state) => {
      const prices = { ...state.prices };
      const tokens = { ...state.tokens };
      const candles = { ...state.candles };
      let candleHistory = state.candleHistory;
      let swaps = state.swaps;
      let signals = state.signals;
      const dexPrices = { ...state.dexPrices };
      let arbOpportunities = state.arbOpportunities;
      const swapSigs = new Set(swaps.map((x) => x.signature));

      for (const msg of messages) {
        switch (msg.type) {
          case 'swap': {
            const s = msg.payload;
            if (swapSigs.has(s.signature)) break;
            swapSigs.add(s.signature);
            if (swaps.length >= MAX_SWAPS) {
              swapSigs.delete(swaps[0].signature);
              swaps = [...swaps.slice(1), s];
            } else {
              swaps = [...swaps, s];
            }

            const amountIn = amountToHuman(s.token_in, s.amount_in);
            const amountOut = amountToHuman(s.token_out, s.amount_out);
            const vol = amountIn;
            const priceOut = swapPriceUsd(
              s.token_in,
              s.token_out,
              s.amount_in,
              s.amount_out,
              prices,
            );
            const priceIn = swapPriceUsd(
              s.token_out,
              s.token_in,
              s.amount_out,
              s.amount_in,
              prices,
            );

            const upsertToken = (
              mint: string,
              price: number,
              isBuy: boolean,
              volAdd: number,
            ) => {
              const prev = tokens[mint];
              const openPrice = prev?.price_usd ?? refPriceForMint(mint, prices[mint]?.price_usd);
              const buyFlow = (prev?.buyFlow ?? 0) + (isBuy ? volAdd : 0);
              const sellFlow = (prev?.sellFlow ?? 0) + (isBuy ? 0 : volAdd);
              const imb =
                buyFlow + sellFlow > 0 ? (buyFlow - sellFlow) / (buyFlow + sellFlow) : 0;
              const changePct =
                openPrice > 0 ? ((price - openPrice) / openPrice) * 100 : 0;
              tokens[mint] = {
                mint,
                price_usd: price,
                volume: (prev?.volume ?? 0) + volAdd,
                buyFlow,
                sellFlow,
                flowImbalance: imb,
                changePct,
                slot: s.slot,
                timestamp_ms: s.timestamp_ms,
              };
              if (!dexPrices[mint]) dexPrices[mint] = {} as Record<Dex, number>;
              dexPrices[mint] = { ...dexPrices[mint], [s.dex]: price };
              const newArb = recomputeArb(dexPrices, mint, s.timestamp_ms);
              if (newArb.length > 0) {
                arbOpportunities = [newArb[0], ...arbOpportunities].slice(0, 30);
              }
            };

            upsertToken(s.token_out, priceOut, true, vol);
            upsertToken(s.token_in, priceIn, false, vol);
            break;
          }
          case 'token_price': {
            const p = msg.payload;
            const priceUsd = sanitizeTradePrice(p.mint, p.price_usd, prices[p.mint]?.price_usd);
            prices[p.mint] = { ...p, price_usd: priceUsd };
            const existing = tokens[p.mint];
            const open = existing?.price_usd ?? refPriceForMint(p.mint, priceUsd);
            const changePct =
              open > 0 ? ((priceUsd - open) / open) * 100 : 0;
            tokens[p.mint] = {
              mint: p.mint,
              price_usd: priceUsd,
              volume: existing?.volume ?? 0,
              buyFlow: existing?.buyFlow ?? 0,
              sellFlow: existing?.sellFlow ?? 0,
              flowImbalance: existing?.flowImbalance ?? 0,
              changePct,
              slot: p.slot,
              timestamp_ms: p.timestamp_ms,
            };
            break;
          }
          case 'candle':
            if (STORE_CANDLES) {
              const raw = msg.payload;
              const live = prices[raw.mint]?.price_usd ?? tokens[raw.mint]?.price_usd;
              const c = normalizeCandle(raw, live);
              const key = candleKey(c.mint, c.interval);
              candles[key] = c;
              candleHistory = pushCandleHistory(candleHistory, key, c);
            }
            break;
          case 'signal':
            signals = [msg.payload, ...signals].slice(0, MAX_SIGNALS);
            break;
        }
      }

      return {
        prices,
        tokens,
        swaps,
        candles,
        candleHistory,
        signals,
        dexPrices,
        arbOpportunities,
        lastSeq: meta.seq,
        lastTsMs: meta.ts_ms,
      };
    });
  },

  mergeArbOpportunities: (external) =>
    set((state) => ({
      arbOpportunities: dedupeArb([...external, ...state.arbOpportunities]),
    })),

  clear: () => set(empty()),
}));
