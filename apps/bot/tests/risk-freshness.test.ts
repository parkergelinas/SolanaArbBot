import { describe, expect, it } from 'vitest';

import { evaluateQuoteFreshness } from '../src/strategy/route-quality.js';
import { computeDynamicSize } from '../src/sizing/dynamic.js';
import { checkStrategyRisk, DEFAULT_RISK_LIMITS } from '../src/risk/limits.js';
import type { TradeDecision } from '../src/strategy/types.js';

describe('evaluateQuoteFreshness', () => {
  it('flags stale forward/reverse skew', () => {
    const now = Date.now();
    const result = evaluateQuoteFreshness(
      now - 2000,
      now - 100,
      { inAmount: '1', outAmount: '1' },
      { inAmount: '1', outAmount: '1' },
      now,
    );
    expect(result.stale).toBe(true);
    expect(result.reasons).toContain('forward_stale');
  });
});

describe('computeDynamicSize', () => {
  it('scales down on low spread and high failure rate', () => {
    const small = computeDynamicSize({
      baseAmountUi: 1,
      spreadBps: 5,
      liquidityUsd: 20_000,
      volatilityPct: 6,
      routeQualityScore: 0.4,
      recentFailureRate: 0.4,
      minAmountUi: 0.1,
      maxAmountUi: 10,
    });
    const large = computeDynamicSize({
      baseAmountUi: 1,
      spreadBps: 60,
      liquidityUsd: 1_000_000,
      volatilityPct: 1,
      routeQualityScore: 0.9,
      recentFailureRate: 0,
      minAmountUi: 0.1,
      maxAmountUi: 10,
    });
    expect(small).toBeLessThan(large);
  });
});

describe('checkStrategyRisk', () => {
  const baseDecision: TradeDecision = {
    strategyId: 'round_trip_quote_arb',
    pairLabel: 'SOL/USDC',
    inputMint: 'So11111111111111111111111111111111111111112',
    outputMint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
    amountInAtomic: '1000000000',
    expectedOutAtomic: '1010000000',
    netProfitUsd: 1,
    grossSpreadBps: 30,
    routeQualityScore: 0.8,
    freshnessScore: 0.9,
  };

  it('halts on session loss limit', () => {
    const result = checkStrategyRisk(
      baseDecision,
      {
        sessionLossUsd: 100,
        consecutiveQuoteFailures: 0,
        inventoryUsd: {},
        routeFamilyFailures: {},
        cooldownUntilMs: 0,
      },
      DEFAULT_RISK_LIMITS,
    );
    expect(result.verdict).toBe('halt');
  });
});
