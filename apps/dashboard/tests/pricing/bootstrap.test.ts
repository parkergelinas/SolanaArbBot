import { beforeEach, describe, expect, it, vi } from 'vitest';

import { bootstrapWatchlistPrices } from '@/lib/pricing/bootstrap';
import { setAnchorPrices, getAnchorPrice } from '@/lib/pricing/anchorCache';
import { SOL_MINT } from '@/lib/terminal/tokens';

vi.mock('@/lib/dexscreener/client', () => ({
  resolvePairForMint: vi.fn(),
}));

import { resolvePairForMint } from '@/lib/dexscreener/client';

const mockResolve = vi.mocked(resolvePairForMint);

describe('bootstrapWatchlistPrices', () => {
  beforeEach(() => {
    mockResolve.mockReset();
    setAnchorPrices({});
  });

  it('builds anchors from DexScreener snapshots', async () => {
    mockResolve.mockImplementation(async (mint) => {
      if (mint === SOL_MINT) {
        return {
          pairAddress: 'abc',
          dexId: 'raydium',
          pairUrl: 'https://dexscreener.com/solana/abc',
          baseSymbol: 'SOL',
          quoteSymbol: 'USDC',
          priceUsd: 64.05,
          changeH24Pct: -6.4,
          volumeH24Usd: 35_000_000,
          liquidityUsd: 1_500_000,
        };
      }
      return null;
    });

    const { anchors, hits } = await bootstrapWatchlistPrices([SOL_MINT]);
    expect(hits).toBeGreaterThan(0);
    expect(anchors[SOL_MINT]?.priceUsd).toBeCloseTo(64.05);
    expect(anchors[SOL_MINT]?.priceUsd).not.toBeCloseTo(145.42);
  });

  it('anchor cache receives injected prices', () => {
    setAnchorPrices({ [SOL_MINT]: 64.05 });
    expect(getAnchorPrice(SOL_MINT)).toBeCloseTo(64.05);
  });
});
