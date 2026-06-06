import type { Candle } from '@/lib/stream/types';

import { getAnchorPrice } from '@/lib/pricing/anchorCache';

export function refPriceForMint(mint: string, livePrice?: number): number {
  if (livePrice !== undefined && Number.isFinite(livePrice) && livePrice > 0) {
    return livePrice;
  }
  const anchor = getAnchorPrice(mint);
  if (anchor && anchor > 0) return anchor;
  // Stablecoins only — never hardcode SOL at $145
  if (mint.startsWith('EPjFW') || mint.startsWith('Es9vM')) return 1;
  return 0;
}

/** Widen guard for swap-implied prices — do NOT use on OHLC candles. */
export function sanitizeTradePrice(mint: string, price: number, livePrice?: number): number {
  const ref = refPriceForMint(mint, livePrice);
  if (!Number.isFinite(price) || price <= 0) return ref > 0 ? ref : price;
  if (ref <= 0) return price;
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
  interval: Candle['interval'],
): ChartBar[] {
  return candles
    .filter((c) => c.interval === interval)
    .sort((a, b) => a.ts_open_ms - b.ts_open_ms)
    .map((c) => ({
      ts: c.ts_open_ms,
      label: new Date(c.ts_open_ms).toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
      }),
      open: c.open,
      high: c.high,
      low: c.low,
      close: c.close,
      volume: c.volume,
    }));
}
