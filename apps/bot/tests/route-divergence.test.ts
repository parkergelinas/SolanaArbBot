import { describe, expect, it } from 'vitest';

import { RouteDivergenceArbStrategy } from '../src/signals/route-divergence-arb.js';
import { MeanReversionStrategy } from '../src/signals/mean-reversion.js';
import type { MarketState, RouteDivergenceSnapshot } from '../src/market/state.js';
import type { JupiterClient } from '../src/jupiter/client.js';

const SOL = 'So11111111111111111111111111111111111111112';
const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

function baseState(div: RouteDivergenceSnapshot): MarketState {
  return {
    timestampMs: Date.now(),
    pricesUsd: { [SOL]: 100, [USDC]: 1 },
    decimals: { [SOL]: 9, [USDC]: 6 },
    quality: {
      [SOL]: {
        verified: true,
        strict: true,
        organicScore: 90,
        liquidityUsd: 1_000_000,
        listingAgeMs: 0,
        priceRecencyMs: 0,
      },
      [USDC]: {
        verified: true,
        strict: true,
        organicScore: 95,
        liquidityUsd: 5_000_000,
        listingAgeMs: 0,
        priceRecencyMs: 0,
      },
    },
    universe: [SOL, USDC],
    solPriceUsd: 100,
    routeDivergence: div,
  };
}

function mockDiv(restrictedOut: string, unrestrictedOut: string): RouteDivergenceSnapshot {
  const now = Date.now();
  const fwdAmount = '1000000000';
  const fwdOut = '150000000';
  return {
    pairLabel: 'SOL/USDC',
    bestConstructionLabel: 'unrestricted',
    divergenceBps: 50,
    constructions: [
      {
        label: 'restricted',
        restrictIntermediateTokens: true,
        forward: {
          request: { inputMint: SOL, outputMint: USDC, amount: fwdAmount },
          response: { inAmount: fwdAmount, outAmount: fwdOut },
          capturedAtMs: now,
        },
        reverse: {
          request: { inputMint: USDC, outputMint: SOL, amount: fwdOut },
          response: { inAmount: fwdOut, outAmount: restrictedOut },
          capturedAtMs: now + 50,
        },
      },
      {
        label: 'unrestricted',
        restrictIntermediateTokens: false,
        forward: {
          request: { inputMint: SOL, outputMint: USDC, amount: fwdAmount },
          response: { inAmount: fwdAmount, outAmount: fwdOut },
          capturedAtMs: now,
        },
        reverse: {
          request: { inputMint: USDC, outputMint: SOL, amount: fwdOut },
          response: { inAmount: fwdOut, outAmount: unrestrictedOut },
          capturedAtMs: now + 50,
        },
      },
    ],
  };
}

describe('RouteDivergenceArbStrategy', () => {
  const stubClient = {} as JupiterClient;
  const strat = new RouteDivergenceArbStrategy(stubClient, 1, {
    minDivergenceBps: 10,
    minSurvivingEdgeBps: 5,
  });

  it('prefers profitable unrestricted construction when divergence is high', () => {
    const decision = strat.evaluate(baseState(mockDiv('1010000000', '1020000000')), {
      minProfitUsd: 0.1,
      slippageBps: 30,
    });
    expect(decision).not.toBeNull();
    expect(decision!.strategyId).toBe('route_divergence_arb');
    expect(decision!.metadata?.divergenceBps).toBe(50);
    expect(decision!.netProfitUsd).toBeGreaterThan(0);
  });

  it('rejects when route divergence is too low', () => {
    const div = mockDiv('1005000000', '1006000000');
    div.divergenceBps = 5;
    const decision = strat.evaluate(baseState(div), { minProfitUsd: 0.1, slippageBps: 30 });
    expect(decision?.rejectionReason).toMatch(/low_route_divergence/);
  });
});

describe('MeanReversionStrategy', () => {
  it('returns scaffold rejection when disabled', () => {
    const strat = new MeanReversionStrategy({ enabled: false, windowSize: 5, entryZScore: 2, exitZScore: 0.5 });
    const state: MarketState = {
      timestampMs: Date.now(),
      pricesUsd: { [SOL]: 100, [USDC]: 1 },
      decimals: { [SOL]: 9, [USDC]: 6 },
      quality: {},
      universe: [SOL, USDC],
      solPriceUsd: 100,
    };
    for (let i = 0; i < 6; i++) {
      state.timestampMs += 1000;
      state.pricesUsd[SOL] = 100 + i * 2;
      const d = strat.evaluate(state, { minProfitUsd: 0.25, slippageBps: 50 });
      if (i >= 4) {
        expect(d?.rejectionReason).toBe('mean_reversion_scaffold');
        expect(d?.metadata?.zScore).toBeDefined();
      }
    }
  });
});
