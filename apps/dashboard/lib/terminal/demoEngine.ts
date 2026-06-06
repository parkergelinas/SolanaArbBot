/**
 * Client-side market simulator — seeds the terminal when stream-api is offline.
 * TradingView-style: always show data; live stream overrides when connected.
 */

import type { Candle, CandleInterval, Dex, Signal, SwapEvent, WSMessage } from '@/lib/stream/types';
import { SCHEMA_VERSION } from '@/lib/stream/types';

import { SOL_MINT, USDC_MINT, WATCHLIST, type WatchToken } from './tokens';

const DEXES: Dex[] = ['raydium', 'orca', 'jupiter'];
const INTERVALS: CandleInterval[] = ['1s', '5s', '1m'];
const INTERVAL_MS: Record<CandleInterval, number> = { '1s': 1000, '5s': 5000, '1m': 60_000 };

/** Random walk price per mint (demo state). */
const livePrices = new Map<string, number>();
let seq = 0;
let slot = 280_412_000;

function initPrices() {
  for (const t of WATCHLIST) {
    if (!livePrices.has(t.mint)) livePrices.set(t.mint, t.refPrice);
  }
}

function bumpPrice(mint: string): number {
  const meta = WATCHLIST.find((t) => t.mint === mint);
  const ref = meta?.refPrice ?? 1;
  const prev = livePrices.get(mint) ?? ref;
  const vol = mint === SOL_MINT ? 0.0018 : mint === USDC_MINT ? 0.0002 : 0.004;
  const next = Math.max(ref * 0.85, Math.min(ref * 1.15, prev * (1 + (Math.random() - 0.48) * vol)));
  livePrices.set(mint, next);
  return next;
}

function rawAmount(token: WatchToken, human: number): string {
  return Math.round(human * 10 ** token.decimals).toString();
}

function buildCandle(
  mint: string,
  interval: CandleInterval,
  open: number,
  close: number,
  volume: number,
  tsOpen: number,
): Candle {
  const spread = Math.abs(close - open) * 0.6 + open * 0.0008;
  return {
    v: SCHEMA_VERSION,
    mint,
    interval,
    open,
    high: Math.max(open, close) + spread * Math.random(),
    low: Math.min(open, close) - spread * Math.random(),
    close,
    volume,
    ts_open_ms: tsOpen,
  };
}

/** Seed ~90 candles per interval for each watchlist token. */
export function seedHistoricalCandles(): WSMessage[] {
  initPrices();
  const now = Date.now();
  const out: WSMessage[] = [];

  for (const token of WATCHLIST) {
    let price = token.refPrice * (0.97 + Math.random() * 0.06);

    for (const interval of INTERVALS) {
      const step = INTERVAL_MS[interval];
      const count = interval === '1m' ? 60 : 90;
      const start = now - count * step;

      for (let i = 0; i < count; i++) {
        const open = price;
        const close = open * (1 + (Math.random() - 0.5) * (interval === '1m' ? 0.012 : 0.004));
        price = close;
        livePrices.set(token.mint, close);
        const vol = (Math.random() * 800 + 50) * (token.mint === SOL_MINT ? 12 : 1);
        out.push({
          type: 'candle',
          payload: buildCandle(token.mint, interval, open, close, vol, start + i * step),
        });
      }
    }

    out.push({
      type: 'token_price',
      payload: {
        v: SCHEMA_VERSION,
        mint: token.mint,
        price_usd: price,
        slot,
        timestamp_ms: now,
      },
    });
  }

  return out;
}

function randomSwap(): WSMessage[] {
  const ts = Date.now();
  const dex = DEXES[seq % DEXES.length];
  const tokenIn = WATCHLIST[seq % WATCHLIST.length];
  const tokenOut = WATCHLIST[(seq + 1 + (seq % 3)) % WATCHLIST.length];
  if (tokenIn.mint === tokenOut.mint) return [];

  const priceIn = bumpPrice(tokenIn.mint);
  const priceOut = bumpPrice(tokenOut.mint);
  const humanIn = tokenIn.mint === SOL_MINT ? 2 + Math.random() * 40 : 500 + Math.random() * 8000;
  const notional = humanIn * priceIn;
  const humanOut = notional / priceOut;

  const swap: SwapEvent = {
    v: SCHEMA_VERSION,
    signature: `demo_${seq.toString(16).padStart(12, '0')}`,
    dex,
    token_in: tokenIn.mint,
    token_out: tokenOut.mint,
    amount_in: rawAmount(tokenIn, humanIn),
    amount_out: rawAmount(tokenOut, humanOut),
    wallet: `Demo${(seq % 9999).toString().padStart(4, '0')}`,
    slot: slot++,
    timestamp_ms: ts,
  };

  const msgs: WSMessage[] = [
    { type: 'swap', payload: swap },
    {
      type: 'token_price',
      payload: {
        v: SCHEMA_VERSION,
        mint: tokenOut.mint,
        price_usd: priceOut,
        slot: swap.slot,
        timestamp_ms: ts,
      },
    },
    {
      type: 'token_price',
      payload: {
        v: SCHEMA_VERSION,
        mint: tokenIn.mint,
        price_usd: priceIn,
        slot: swap.slot,
        timestamp_ms: ts,
      },
    },
  ];

  const notionalUsd = humanIn * priceIn;
  const volOut = Math.max(10, notionalUsd * (0.15 + Math.random() * 0.25));

  for (const interval of INTERVALS) {
    const step = INTERVAL_MS[interval];
    const tsOpen = Math.floor(ts / step) * step;
    const prev = livePrices.get(tokenOut.mint) ?? priceOut;
    const open = prev * (1 + (Math.random() - 0.5) * 0.001);
    msgs.push({
      type: 'candle',
      payload: buildCandle(tokenOut.mint, interval, open, priceOut, volOut, tsOpen),
    });
    if (tokenIn.mint === SOL_MINT || tokenOut.mint === SOL_MINT) {
      const solPx = livePrices.get(SOL_MINT) ?? 145;
      const solOpen = solPx * (1 + (Math.random() - 0.5) * 0.0008);
      const solVol = tokenIn.mint === SOL_MINT ? humanIn * priceIn : humanOut * priceOut;
      msgs.push({
        type: 'candle',
        payload: buildCandle(SOL_MINT, interval, solOpen, solPx, Math.max(50, solVol), tsOpen),
      });
    }
  }

  if (seq % 7 === 0) {
    const kinds: Signal['kind'][] = ['momentum', 'whale_flow', 'smart_money', 'imbalance'];
    msgs.push({
      type: 'signal',
      payload: {
        v: SCHEMA_VERSION,
        signal_id: `demo_sig_${seq}`,
        mint: tokenOut.mint,
        kind: kinds[seq % kinds.length],
        strength: 0.4 + Math.random() * 0.55,
        confidence: 0.55 + Math.random() * 0.4,
        timestamp_ms: ts,
        detail: `${dex} flow · ${tokenIn.symbol}→${tokenOut.symbol}`,
      },
    });
  }

  seq++;
  return msgs;
}

export type DemoEngineHandle = { stop: () => void };

/** Run demo ticks until `stop()`; intervalMs default 120. */
export function startDemoEngine(
  onBatch: (messages: WSMessage[], meta: { seq: number; ts_ms: number }) => void,
  options?: { intervalMs?: number; seed?: boolean },
): DemoEngineHandle {
  const intervalMs = options?.intervalMs ?? 120;
  let batchSeq = 0;
  let timer: ReturnType<typeof setInterval> | null = null;

  initPrices();

  if (options?.seed !== false) {
    const seed = seedHistoricalCandles();
    batchSeq++;
    onBatch(seed, { seq: batchSeq, ts_ms: Date.now() });
  }

  timer = setInterval(() => {
    const msgs = randomSwap();
    if (msgs.length === 0) return;
    batchSeq++;
    onBatch(msgs, { seq: batchSeq, ts_ms: Date.now() });
  }, intervalMs);

  return {
    stop: () => {
      if (timer) clearInterval(timer);
      timer = null;
    },
  };
}
