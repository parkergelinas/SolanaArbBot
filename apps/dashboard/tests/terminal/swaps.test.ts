import { describe, expect, it } from 'vitest';

import { formatCompact } from '@/lib/format/numbers';
import { swapDirection } from '@/lib/terminal/swaps';
import { makeSwap } from '../helpers/fixtures';

const SOL = 'So11111111111111111111111111111111111111112';
const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

describe('formatCompact', () => {
  it('formats large values with K/M suffixes', () => {
    expect(formatCompact(1_500)).toBe('1.5K');
    expect(formatCompact(2_500_000)).toBe('2.5M');
  });

  it('formats small decimals', () => {
    expect(formatCompact(0.0042)).toBe('0.0042');
  });
});

describe('swapDirection', () => {
  it('treats SOL in as buy', () => {
    expect(swapDirection(makeSwap({ token_in: SOL, token_out: USDC }))).toBe('buy');
  });

  it('treats SOL out as sell', () => {
    expect(swapDirection(makeSwap({ token_in: USDC, token_out: SOL }))).toBe('sell');
  });
});
