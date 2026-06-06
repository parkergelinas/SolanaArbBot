import { describe, expect, it } from 'vitest';

import { isPumpFunPair, pairAgeMinutes, type DexPair } from '@/lib/scanners/dexscreener';

describe('pump.fun scanner helpers', () => {
  it('detects pumpfun dex id', () => {
    expect(isPumpFunPair({ dexId: 'pumpfun', chainId: 'solana' })).toBe(true);
    expect(isPumpFunPair({ dexId: 'raydium', chainId: 'solana' })).toBe(false);
    expect(isPumpFunPair({ labels: ['pump'], chainId: 'solana' })).toBe(true);
  });

  it('computes pair age in minutes', () => {
    const now = Date.now();
    const pair: DexPair = { pairCreatedAt: now - 30 * 60_000 };
    expect(pairAgeMinutes(pair, now)).toBeCloseTo(30, 0);
  });
});
