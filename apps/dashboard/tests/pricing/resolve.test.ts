import { describe, expect, it } from 'vitest';

import { resolveTokenPrice } from '@/lib/pricing/resolve';
import { SOL_MINT } from '@/lib/terminal/tokens';

const SOL = SOL_MINT;

const anchor64 = {
  priceUsd: 64,
  changeH24Pct: -1,
  volumeH24Usd: 1e6,
  liquidityUsd: 1e6,
  fetchedAt: Date.now(),
};

const stream145 = {
  mint: SOL,
  price_usd: 145,
  volume: 0,
  buyFlow: 0,
  sellFlow: 0,
  flowImbalance: 0,
  changePct: 0,
  slot: 1,
  timestamp_ms: Date.now(),
};

describe('resolveTokenPrice', () => {
  it('prefers fresh DexScreener over stream and anchor', () => {
    const r = resolveTokenPrice({
      mint: SOL,
      connectionMode: 'live',
      dex: {
        pairAddress: 'x',
        dexId: 'raydium',
        pairUrl: 'https://example.com',
        baseSymbol: 'SOL',
        quoteSymbol: 'USDC',
        priceUsd: 64.05,
        changeH24Pct: -6.4,
        volumeH24Usd: 35_000_000,
        liquidityUsd: 1_500_000,
      },
      anchor: anchor64,
      streamToken: stream145,
    });
    expect(r.priceUsd).toBeCloseTo(64.05);
    expect(r.source).toBe('dex');
    expect(r.stale).toBe(false);
  });

  it('uses anchor before stale sim stream in sim mode', () => {
    const r = resolveTokenPrice({
      mint: SOL,
      connectionMode: 'sim',
      anchor: anchor64,
      streamToken: stream145,
    });
    expect(r.priceUsd).toBeCloseTo(64);
    expect(r.source).toBe('dex');
  });

  it('uses anchor before divergent live stream (watchlist/navbar sync)', () => {
    const r = resolveTokenPrice({
      mint: SOL,
      connectionMode: 'live',
      anchor: anchor64,
      streamToken: stream145,
    });
    expect(r.priceUsd).toBeCloseTo(64);
    expect(r.source).toBe('dex');
  });

  it('uses anchor in degraded mode before bad stream', () => {
    const r = resolveTokenPrice({
      mint: SOL,
      connectionMode: 'degraded',
      anchor: anchor64,
      streamToken: stream145,
    });
    expect(r.priceUsd).toBeCloseTo(64);
    expect(r.source).toBe('dex');
  });

  it('uses live stream when consistent with anchor', () => {
    const r = resolveTokenPrice({
      mint: SOL,
      connectionMode: 'live',
      anchor: anchor64,
      streamToken: {
        ...stream145,
        price_usd: 64.2,
        changePct: 0.5,
      },
    });
    expect(r.priceUsd).toBeCloseTo(64.2);
    expect(r.source).toBe('stream');
  });

  it('uses live stream when no anchor', () => {
    const r = resolveTokenPrice({
      mint: SOL,
      connectionMode: 'live',
      streamToken: {
        ...stream145,
        price_usd: 64.2,
      },
    });
    expect(r.priceUsd).toBeCloseTo(64.2);
    expect(r.source).toBe('stream');
  });

  it('falls back to sim stream when no dex or anchor', () => {
    const r = resolveTokenPrice({
      mint: SOL,
      connectionMode: 'sim',
      streamToken: {
        ...stream145,
        price_usd: 62.5,
      },
    });
    expect(r.priceUsd).toBeCloseTo(62.5);
    expect(r.source).toBe('sim');
    expect(r.stale).toBe(true);
  });

  it('returns none when no sources available', () => {
    const r = resolveTokenPrice({ mint: SOL, connectionMode: 'sim' });
    expect(r.priceUsd).toBe(0);
    expect(r.source).toBe('none');
  });
});
