import { describe, expect, it } from 'vitest';

import { resolvePairForMint } from '@/lib/dexscreener/client';
import { SOL_MINT } from '@/lib/terminal/tokens';

describe('SOL live DexScreener anchor', () => {
  it('returns USD price in reasonable range (not stale $145)', async () => {
    const snap = await resolvePairForMint(SOL_MINT);
    expect(snap).not.toBeNull();
    expect(snap!.priceUsd).toBeGreaterThan(50);
    expect(snap!.priceUsd).toBeLessThan(200);
    expect(snap!.priceUsd).not.toBeCloseTo(145.42);
  }, 15_000);
});
