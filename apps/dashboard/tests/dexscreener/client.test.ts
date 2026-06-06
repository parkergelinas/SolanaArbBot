import { describe, expect, it } from 'vitest';

import {
  dexScreenerEmbedUrl,
  pickBestPair,
  pairToSnapshot,
} from '@/lib/dexscreener/client';
import type { DexPair } from '@/lib/dexscreener/types';

const SOL = 'So11111111111111111111111111111111111111112';
const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

function pair(partial: Partial<DexPair> & { pairAddress: string }): DexPair {
  return {
    chainId: 'solana',
    dexId: 'raydium',
    url: `https://dexscreener.com/solana/${partial.pairAddress}`,
    baseToken: { address: SOL, symbol: 'SOL' },
    quoteToken: { address: USDC, symbol: 'USDC' },
    priceUsd: '145.5',
    volume: { h24: 1_000_000 },
    priceChange: { h24: 2.5 },
    liquidity: { usd: 10_000 },
    ...partial,
  };
}

describe('pickBestPair', () => {
  it('selects highest liquidity pair for mint', () => {
    const best = pickBestPair(
      [
        pair({ pairAddress: 'low', liquidity: { usd: 5_000 } }),
        pair({ pairAddress: 'high', liquidity: { usd: 500_000 } }),
      ],
      SOL,
    );
    expect(best?.pairAddress).toBe('high');
  });
});

describe('dexScreenerEmbedUrl', () => {
  it('builds embed URL with dark theme', () => {
    const url = dexScreenerEmbedUrl('abc123');
    expect(url).toContain('https://dexscreener.com/solana/abc123');
    expect(url).toContain('embed=1');
    expect(url).toContain('theme=dark');
  });
});

describe('pairToSnapshot', () => {
  it('maps API pair to snapshot', () => {
    const snap = pairToSnapshot(pair({ pairAddress: 'x' }));
    expect(snap.priceUsd).toBeCloseTo(145.5);
    expect(snap.changeH24Pct).toBe(2.5);
    expect(snap.baseSymbol).toBe('SOL');
  });
});
