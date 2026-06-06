import type { Candle } from '@/lib/stream/types';

import { tokenMeta } from './tokens';

export function refPriceForMint(mint: string, livePrice?: number): number {
  if (livePrice !== undefined && Number.isFinite(livePrice) && livePrice > 0) {
    return livePrice;
  }
  return tokenMeta(mint)?.refPrice ?? (mint.startsWith('So1111') ? 145 : 1);
}

/** Widen guard for swap-implied prices — do NOT use on OHLC candles. */
export function sanitizeTradePrice(mint: string, price: number, livePrice?: number): number {
  const ref = refPriceForMint(mint, livePrice);
  if (!Number.isFinite(price) || price <= 0) return ref;
  if (price > ref * 100 || price < ref / 100) return ref;
  return price;
}

/** Keep OHLC internally consistent; never clamp micro-cap tokens to $1. */
export function normalizeCandle(raw: Candle, fallbackPrice?: number): Candle {
  const fb = fallbackPrice ?? refPriceForMint(raw.mint);
  const fin = (n: number, d: number) => (Number.isFinite(n) && n > 0 ? n : d);

  let open = fin(raw.open, fb);
  let close = fin(raw.close, open);
  let high = fin(raw.high, Math.max(open, close));
  let low = fin(raw.low, Math.min(open, close));

  high = Math.max(high, open, close);
  low = Math.min(low, open, close);

  const minRange = Math.max(Math.abs(open) * 0.00005, 1e-12);
  if (high - low < minRange) {
    const mid = (high + low) / 2;
    high = mid + minRange / 2;
    low = mid - minRange / 2;
  }

  return {
    ...raw,
    open,
    high,
    low,
    close,
    volume: Number.isFinite(raw.volume) && raw.volume >= 0 ? raw.volume : 0,
  };
}

export interface ChartBar {
  ts: number;
  label: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export function prepareChartBars(
  candles: Candle[],
  interval: '1s' | '5s' | '1m',
  maxBars = 96,
): ChartBar[] {
  const sorted = [...candles].sort((a, b) => a.ts_open_ms - b.ts_open_ms);
  const slice = sorted.slice(-maxBars);

  return slice.map((c) => {
    const n = normalizeCandle(c);
    return {
      ts: n.ts_open_ms,
      label:
        interval === '1m'
          ? new Date(n.ts_open_ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
          : new Date(n.ts_open_ms).toLocaleTimeString([], {
              hour: '2-digit',
              minute: '2-digit',
              second: '2-digit',
            }),
      open: n.open,
      high: n.high,
      low: n.low,
      close: n.close,
      volume: n.volume,
    };
  });
}
