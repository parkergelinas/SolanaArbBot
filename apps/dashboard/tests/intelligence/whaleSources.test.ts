import { describe, expect, it } from 'vitest';

import {
  classifyWalletTier,
  isSmartMoneySwap,
  isWhaleSwap,
  WHALE_THRESHOLD_SOL,
} from '@/lib/intelligence/whaleSources';

describe('whaleSources', () => {
  it('classifies labeled whale regardless of size', () => {
    expect(classifyWalletTier(1, 'whale')).toBe('whale');
    expect(isWhaleSwap(1, 'whale')).toBe(true);
  });

  it('classifies by SOL threshold', () => {
    expect(classifyWalletTier(WHALE_THRESHOLD_SOL)).toBe('whale');
    expect(isWhaleSwap(WHALE_THRESHOLD_SOL)).toBe(true);
    expect(classifyWalletTier(5)).toBe('active');
    expect(classifyWalletTier(1)).toBe('retail');
  });

  it('detects smart money only with label + min size', () => {
    expect(isSmartMoneySwap(2, 'smart')).toBe(true);
    expect(isSmartMoneySwap(0.5, 'smart')).toBe(false);
    expect(isSmartMoneySwap(5, null)).toBe(false);
  });
});
