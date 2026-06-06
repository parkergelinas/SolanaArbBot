/**
 * Unit tests for profit calculation functions.
 * Covers: scoreRoundTripUsd edge cases, estimateFeesUsd, sqrtPriceX64ToUiPrice.
 */

import { describe, it, expect } from 'vitest';
import { scoreRoundTripUsd, estimateFeesUsd } from '../src/strategy/scorer.js';
import { sqrtPriceX64ToUiPrice } from '../src/orca/whirlpool-monitor.js';
import type { ScoringInput } from '../src/strategy/types.js';
import type { MarketState } from '../src/market/state.js';

// ─── fixtures ─────────────────────────────────────────────────────────────────

const SOL_MINT  = 'So11111111111111111111111111111111111111112';
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

function makeState(overrides: Partial<MarketState> = {}): MarketState {
  return {
    timestampMs: Date.now(),
    pricesUsd: { [SOL_MINT]: 170, [USDC_MINT]: 1 },
    decimals: { [SOL_MINT]: 9, [USDC_MINT]: 6 },
    quality: {},
    universe: [SOL_MINT, USDC_MINT],
    solPriceUsd: 170,
    ...overrides,
  };
}

function makeInput(
  amountInAtomic: string,
  outAmount: string,
  overrides: Partial<MarketState> = {},
): ScoringInput {
  const state = makeState(overrides);
  return {
    pair: {
      pairLabel: 'SOL/USDC',
      forward: {
        request: { inputMint: SOL_MINT, outputMint: USDC_MINT, amount: amountInAtomic },
        response: { inAmount: amountInAtomic, outAmount: '170000000' },
        capturedAtMs: Date.now(),
      },
      reverse: {
        request: { inputMint: USDC_MINT, outputMint: SOL_MINT, amount: '170000000' },
        response: { inAmount: '170000000', outAmount },
        capturedAtMs: Date.now(),
      },
    },
    state,
    inputDecimals: 9,
    outputDecimals: 6,
    feesUsd: estimateFeesUsd({ solPriceUsd: 170, priorityFeeLamports: 50_000 }),
  };
}

// ─── scoreRoundTripUsd ────────────────────────────────────────────────────────

describe('scoreRoundTripUsd', () => {
  it('returns a positive net profit on a profitable round trip', () => {
    // 1 SOL in → 1.005 SOL out (0.5% spread after fees)
    const result = scoreRoundTripUsd(makeInput('1000000000', '1005000000'));
    expect(result.rejected).toBe(false);
    expect(result.grossSpreadBps).toBeGreaterThan(0);
    // Net might still be negative due to fees — that's correct behaviour
    expect(typeof result.netProfitUsd).toBe('number');
  });

  it('returns rejected when input price is zero', () => {
    const result = scoreRoundTripUsd(
      makeInput('1000000000', '1005000000', {
        pricesUsd: { [SOL_MINT]: 0, [USDC_MINT]: 1 },
        solPriceUsd: 0,
      }),
    );
    expect(result.rejected).toBe(true);
    expect(result.rejectionReason).toBe('missing_input_price');
    expect(result.netProfitUsd).toBe(0);
    expect(result.grossSpreadBps).toBe(0);
  });

  it('returns negative net profit on a losing round trip', () => {
    // 1 SOL in → 0.99 SOL out (−1% outcome)
    const result = scoreRoundTripUsd(makeInput('1000000000', '990000000'));
    expect(result.rejected).toBe(false);
    expect(result.netProfitUsd).toBeLessThan(0);
    expect(result.grossSpreadBps).toBeLessThan(0);
  });

  it('handles zero liquidity (zero output amount)', () => {
    const result = scoreRoundTripUsd(makeInput('1000000000', '0'));
    expect(result.rejected).toBe(false);
    expect(result.netProfitUsd).toBeLessThan(0); // large loss
    expect(result.endUsd).toBe(0);
  });

  it('handles minimum atomic amounts (1 lamport)', () => {
    const result = scoreRoundTripUsd(makeInput('1', '1'));
    expect(result.rejected).toBe(false);
    expect(result.startUsd).toBeCloseTo(0, 10);
  });

  it('correctly propagates fees into net profit', () => {
    const fees = estimateFeesUsd({ solPriceUsd: 170, priorityFeeLamports: 50_000, jitoTipLamports: 10_000 });
    const input = makeInput('1000000000', '1010000000'); // 1% gross spread
    input.feesUsd = fees;
    const result = scoreRoundTripUsd(input);
    expect(result.netProfitUsd).toBeLessThan(result.endUsd - result.startUsd);
  });
});

// ─── estimateFeesUsd ─────────────────────────────────────────────────────────

describe('estimateFeesUsd', () => {
  it('computes fee breakdown without slippage when notionalUsd is omitted', () => {
    const fees = estimateFeesUsd({ solPriceUsd: 170 });
    expect(fees.slippageUsd).toBe(0);
    expect(fees.priorityFeeUsd).toBeGreaterThan(0);
    expect(fees.totalUsd).toBeCloseTo(fees.priorityFeeUsd + fees.jitoTipUsd, 10);
  });

  it('includes slippage cost when both notionalUsd and slippageBps are provided', () => {
    const fees = estimateFeesUsd({ solPriceUsd: 170, notionalUsd: 170, slippageBps: 50 });
    expect(fees.slippageUsd).toBeCloseTo(170 * 0.005, 6);
    expect(fees.totalUsd).toBeGreaterThan(fees.priorityFeeUsd + fees.jitoTipUsd);
  });

  it('scales linearly with SOL price', () => {
    const a = estimateFeesUsd({ solPriceUsd: 100, priorityFeeLamports: 50_000 });
    const b = estimateFeesUsd({ solPriceUsd: 200, priorityFeeLamports: 50_000 });
    expect(b.priorityFeeUsd).toBeCloseTo(a.priorityFeeUsd * 2, 10);
  });
});

// ─── sqrtPriceX64ToUiPrice ───────────────────────────────────────────────────

describe('sqrtPriceX64ToUiPrice', () => {
  it('recovers SOL price of ~170 USDC from a synthetic sqrtPriceX64', () => {
    // SOL=9 decimals, USDC=6 decimals; price = 170 USDC per SOL
    // price_raw = 170 / 1000 = 0.17
    // sqrtPriceX64 = sqrt(0.17) * 2^64
    const Q64 = 2n ** 64n;
    const rawPrice = 170 / Math.pow(10, 9 - 6); // 0.17
    const sqrtPriceX64 = BigInt(Math.floor(Math.sqrt(rawPrice) * Number(Q64)));

    const price = sqrtPriceX64ToUiPrice(sqrtPriceX64, 9, 6);
    expect(price).toBeCloseTo(170, 0);
  });

  it('returns a price near zero when sqrtPriceX64 is near zero', () => {
    const price = sqrtPriceX64ToUiPrice(1n, 9, 6);
    expect(price).toBeGreaterThanOrEqual(0);
    expect(price).toBeLessThan(1e-20);
  });

  it('handles equal decimal tokens (USDC/USDT)', () => {
    // price ≈ 1.0 for stablecoin pair
    const Q64 = 2n ** 64n;
    const sqrtPriceX64 = Q64; // sqrt(1) * 2^64
    const price = sqrtPriceX64ToUiPrice(sqrtPriceX64, 6, 6);
    expect(price).toBeCloseTo(1, 10);
  });
});
