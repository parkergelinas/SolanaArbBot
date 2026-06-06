import { describe, expect, it } from 'vitest';

import { isPumpFunPair } from '@/lib/scanners/dexscreener';

describe('solscan researcher shared filters', () => {
  it('excludes non-pump pairs from pump filter', () => {
    expect(isPumpFunPair({ dexId: 'orca', chainId: 'solana' })).toBe(false);
  });
});
