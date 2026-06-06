import { describe, expect, it, vi } from 'vitest';

import { arbOpportunitiesToSignals } from '@/lib/arb/arbSignals';
import type { ArbOpportunity } from '@/stores/marketStore';

describe('arb signals', () => {
  it('converts opportunity to Arb signal with profit estimate', () => {
    const opp: ArbOpportunity = {
      id: 'test-1',
      token: 'mint123',
      symbol: 'SOL',
      buyDex: 'raydium',
      sellDex: 'orca',
      buyPrice: 100,
      sellPrice: 100.5,
      spreadBps: 50,
      timestamp_ms: Date.now(),
      source: 'dexscreener',
      estimatedProfitUsd: 1.2,
      winProbability: 0.72,
      venueCount: 3,
    };
    const sig = arbOpportunitiesToSignals([opp])[0];
    expect(sig.signal_type).toBe('Arb');
    expect(sig.explanation).toContain('cross-DEX');
    expect(sig.confidence).toBeCloseTo(0.72);
  });
});

vi.mock('@/lib/scanners/dexscreener', () => ({
  fetchTokenPairs: vi.fn(async () => []),
}));
