/**
 * Client-side market simulator — seeds the terminal when stream-api is offline.
 * Prices anchor to DexScreener bootstrap, not hardcoded refPrice.
 */

import { getAnchorPrice, setAnchorPrices as setCacheAnchors } from '@/lib/pricing/anchorCache';
import type { Dex, Signal, SwapEvent, WSMessage } from '@/lib/stream/types';
import { SCHEMA_VERSION } from '@/lib/stream/types';

import { SOL_MINT, USDC_MINT, USDT_MINT, WATCHLIST, type WatchToken } from './tokens';

const DEXES: Dex[] = ['raydium', 'orca', 'jupiter'];

const livePrices = new Map<string, number>();
let seq = 0;
let slot = 280_412_000;

export function setDemoPriceAnchors(map: Record<string, number>): void {
  setCacheAnchors(map);
  for (const [mint, price] of Object.entries(map)) {
    if (price > 0) livePrices.set(mint, price);
  }
}

function anchorFor(mint: string, token: WatchToken): number {
  const cached = getAnchorPrice(mint);
  if (cached && cached > 0) return cached;
  if (token.refPrice > 0) return token.refPrice;
  if (mint === USDC_MINT || mint === USDT_MINT) return 1;
  return 1;
}

function initPrices() {
  for (const t of WATCHLIST) {
    if (!livePrices.has(t.mint)) livePrices.set(t.mint, anchorFor(t.mint, t));
  }
}

function bumpPrice(mint: string): number {
  const meta = WATCHLIST.find((t) => t.mint === mint);
  const ref = meta ? anchorFor(mint, meta) : 1;
  const prev = livePrices.get(mint) ?? ref;
  const vol = mint === SOL_MINT ? 0.0018 : 0.004;
  const next = Math.max(ref * 0.85, Math.min(ref * 1.15, prev * (1 + (Math.random() - 0.48) * vol)));
  livePrices.set(mint, next);
  return next;
}

function rawAmount(token: WatchToken, human: number): string {
  return Math.round(human * 10 ** token.decimals).toString();
}

/** Seed watchlist token prices from Dex anchors (±2% jitter). */
export function seedDemoPrices(): WSMessage[] {
  initPrices();
  const now = Date.now();
  return WATCHLIST.map((token) => {
    const anchor = anchorFor(token.mint, token);
    const price = anchor * (0.98 + Math.random() * 0.04);
    livePrices.set(token.mint, price);
    return {
      type: 'token_price' as const,
      payload: {
        v: SCHEMA_VERSION,
        mint: token.mint,
        price_usd: price,
        slot,
        timestamp_ms: now,
      },
    };
  });
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

export function startDemoEngine(
  onBatch: (messages: WSMessage[], meta: { seq: number; ts_ms: number }) => void,
  options?: { intervalMs?: number; seed?: boolean },
): DemoEngineHandle {
  const intervalMs = options?.intervalMs ?? 150;
  let batchSeq = 0;
  let timer: ReturnType<typeof setInterval> | null = null;

  initPrices();

  if (options?.seed !== false) {
    const seed = seedDemoPrices();
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
