import { describe, expect, it } from 'vitest';

import { normalizeCandle, prepareChartBars, sanitizeTradePrice } from '@/lib/terminal/candles';
import type { Candle } from '@/lib/stream/types';

const BONK = 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263';

function candle(partial: Partial<Candle> & Pick<Candle, 'mint'>): Candle {
  return {
    v: 1,
    interval: '1s',
    open: 0.000024,
    high: 0.000025,
    low: 0.000023,
    close: 0.0000245,
    volume: 100,
    ts_open_ms: 1_700_000_000_000,
    ...partial,
  };
}

describe('normalizeCandle', () => {
  it('preserves micro-cap prices instead of clamping to $1', () => {
    const n = normalizeCandle(candle({ mint: BONK }));
    expect(n.close).toBeLessThan(0.001);
    expect(n.high).toBeGreaterThanOrEqual(n.close);
    expect(n.low).toBeLessThanOrEqual(n.open);
  });

  it('repairs inverted high/low', () => {
    const n = normalizeCandle(
      candle({ mint: BONK, high: 0.00001, low: 0.00003, open: 0.000024, close: 0.000024 }),
    );
    expect(n.high).toBeGreaterThanOrEqual(n.low);
  });
});

describe('sanitizeTradePrice', () => {
  it('allows bonk-range prices', () => {
    expect(sanitizeTradePrice(BONK, 0.000024)).toBeCloseTo(0.000024, 8);
  });
});

describe('prepareChartBars', () => {
  it('sorts candles chronologically', () => {
    const bars = prepareChartBars(
      [
        candle({ mint: BONK, ts_open_ms: 3000, close: 0.00003 }),
        candle({ mint: BONK, ts_open_ms: 1000, close: 0.00002 }),
        candle({ mint: BONK, ts_open_ms: 2000, close: 0.000025 }),
      ],
      '1s',
    );
    expect(bars[0].ts).toBe(1000);
    expect(bars[2].ts).toBe(3000);
  });
});
