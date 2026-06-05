import { create } from 'zustand';

import type {
  Candle,
  CandleInterval,
  Dex,
  Signal,
  SwapEvent,
  TokenPrice,
  WSMessage,
} from '@/lib/stream/types';

const MAX_SWAPS = 500;
const MAX_SIGNALS = 80;
const MAX_CANDLE_HISTORY = 120;

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

export interface ArbOpportunity {
  id: string;
  token: string;
  buyDex: Dex;
  sellDex: Dex;
  buyPrice: number;
  sellPrice: number;
  spreadBps: number;
  timestamp_ms: number;
}

interface TokenAccumulator {
  buyFlow: number;
  sellFlow: number;
  volume: number;
  openPrice: number | null;
  lastPrice: number;
}

interface MarketState {
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
  clear: () => void;
}

const empty = (): Omit<MarketState, 'applyMessages' | 'clear'> => ({
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

function parseAmount(s: string): number {
  const n = Number(s);
  return Number.isFinite(n) ? n : 0;
}

function impliedPrice(amountIn: number, amountOut: number, refIn: number): number {
  if (amountOut <= 0) return refIn;
  return (amountIn / amountOut) * refIn;
}

function refPrice(mint: string): number {
  if (mint.startsWith('So1111')) return 145;
  return 1;
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
  } else {
    next = [...prev, candle];
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
    },
  ];
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

      for (const msg of messages) {
        switch (msg.type) {
          case 'swap': {
            const s = msg.payload;
            swaps =
              swaps.length >= MAX_SWAPS
                ? [...swaps.slice(1), s]
                : [...swaps, s];

            const amountIn = parseAmount(s.amount_in);
            const amountOut = parseAmount(s.amount_out);
            const vol = amountIn / 1_000_000;
            const priceOut = impliedPrice(amountIn, amountOut, refPrice(s.token_in));

            const tokenAcc: TokenAccumulator = {
              buyFlow: (tokens[s.token_out]?.buyFlow ?? 0) + vol,
              sellFlow: tokens[s.token_out]?.sellFlow ?? 0,
              volume: (tokens[s.token_out]?.volume ?? 0) + vol,
              openPrice: tokens[s.token_out]?.price_usd ?? priceOut,
              lastPrice: priceOut,
            };
            const imb =
              tokenAcc.buyFlow + tokenAcc.sellFlow > 0
                ? (tokenAcc.buyFlow - tokenAcc.sellFlow) /
                  (tokenAcc.buyFlow + tokenAcc.sellFlow)
                : 0;
            const changePct =
              tokenAcc.openPrice && tokenAcc.openPrice > 0
                ? ((priceOut - tokenAcc.openPrice) / tokenAcc.openPrice) * 100
                : 0;

            tokens[s.token_out] = {
              mint: s.token_out,
              price_usd: priceOut,
              volume: tokenAcc.volume,
              buyFlow: tokenAcc.buyFlow,
              sellFlow: tokenAcc.sellFlow,
              flowImbalance: imb,
              changePct,
              slot: s.slot,
              timestamp_ms: s.timestamp_ms,
            };

            if (!dexPrices[s.token_out]) dexPrices[s.token_out] = {} as Record<Dex, number>;
            dexPrices[s.token_out] = { ...dexPrices[s.token_out], [s.dex]: priceOut };
            const newArb = recomputeArb(dexPrices, s.token_out, s.timestamp_ms);
            if (newArb.length > 0) {
              arbOpportunities = [newArb[0], ...arbOpportunities].slice(0, 30);
            }
            break;
          }
          case 'token_price': {
            const p = msg.payload;
            prices[p.mint] = p;
            const existing = tokens[p.mint];
            const open = existing?.price_usd ?? p.price_usd;
            const changePct =
              open > 0 ? ((p.price_usd - open) / open) * 100 : 0;
            tokens[p.mint] = {
              mint: p.mint,
              price_usd: p.price_usd,
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
          case 'candle': {
            const c = msg.payload;
            const key = candleKey(c.mint, c.interval);
            candles[key] = c;
            candleHistory = pushCandleHistory(candleHistory, key, c);
            break;
          }
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

  clear: () => set(empty()),
}));
