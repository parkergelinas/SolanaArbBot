import { describe, expect, it } from 'vitest';

import { estimateFeesUsd, scoreRoundTripUsd } from '../src/strategy/scorer.js';
import type { QuotePairSnapshot } from '../src/jupiter/types.js';
import type { MarketState } from '../src/market/state.js';

const SOL = 'So11111111111111111111111111111111111111112';
const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

function mockPair(forwardOut: string, reverseOut: string): QuotePairSnapshot {
  return {
    pairLabel: 'SOL/USDC',
    forward: {
      request: { inputMint: SOL, outputMint: USDC, amount: '1000000000' },
      response: { inAmount: '1000000000', outAmount: forwardOut },
      capturedAtMs: Date.now(),
    },
    reverse: {
      request: { inputMint: USDC, outputMint: SOL, amount: forwardOut },
      response: { inAmount: forwardOut, outAmount: reverseOut },
      capturedAtMs: Date.now(),
    },
  };
}

function mockState(solPrice = 100): MarketState {
  return {
    timestampMs: Date.now(),
    pricesUsd: { [SOL]: solPrice, [USDC]: 1 },
    decimals: { [SOL]: 9, [USDC]: 6 },
    quality: {},
    universe: [SOL, USDC],
    solPriceUsd: solPrice,
  };
}

describe('scoreRoundTripUsd', () => {
  it('computes profit entirely in USD (no unit mixing)', () => {
    const pair = mockPair('150000000', '1010000000');
    const fees = estimateFeesUsd({
      solPriceUsd: 100,
      priorityFeeLamports: 50_000,
      jitoTipLamports: 10_000,
      slippageBps: 30,
      notionalUsd: 100,
    });

    const result = scoreRoundTripUsd({
      pair,
      state: mockState(100),
      inputDecimals: 9,
      outputDecimals: 6,
      feesUsd: fees,
    });

    expect(result.startUsd).toBeCloseTo(100, 1);
    expect(result.endUsd).toBeCloseTo(101, 1);
    expect(result.netProfitUsd).toBeCloseTo(result.endUsd - result.startUsd - fees.totalUsd, 4);
    expect(result.grossSpreadBps).toBeGreaterThan(0);
  });

  it('rejects when input price missing', () => {
    const state = mockState(0);
    state.pricesUsd[SOL] = 0;
    const result = scoreRoundTripUsd({
      pair: mockPair('150000000', '1000000000'),
      state,
      inputDecimals: 9,
      outputDecimals: 6,
      feesUsd: estimateFeesUsd({ solPriceUsd: 0 }),
    });
    expect(result.rejected).toBe(true);
  });
});

describe('estimateFeesUsd', () => {
  it('returns all fee components in USD', () => {
    const fees = estimateFeesUsd({
      solPriceUsd: 100,
      priorityFeeLamports: 1_000_000,
      jitoTipLamports: 500_000,
      slippageBps: 50,
      notionalUsd: 200,
    });
    expect(fees.priorityFeeUsd).toBeGreaterThan(0);
    expect(fees.totalUsd).toBe(
      fees.priorityFeeUsd + fees.jitoTipUsd + fees.slippageUsd,
    );
  });
});
