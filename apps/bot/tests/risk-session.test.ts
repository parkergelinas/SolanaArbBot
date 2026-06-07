/**
 * Risk session lifecycle tests.
 *
 * Covers the stateful layer between the engine scan loop and execution:
 *   checkStrategyRisk  → gate each trade
 *   recordExecutionOutcome → accumulate state after each trade
 *
 * These paths are exercised by both the scan-loop and the event-driven
 * handleExternalOpportunity, so correctness here is critical for both.
 */

import { describe, expect, it } from 'vitest';

import {
  checkStrategyRisk,
  DEFAULT_RISK_LIMITS,
  recordExecutionOutcome,
  type StrategyRiskState,
} from '../src/risk/index.js';
import type { TradeDecision } from '../src/strategy/types.js';

// ── Helpers ──────────────────────────────────────────────────────────────────

const DECISION: TradeDecision = {
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

function freshState(): StrategyRiskState {
  return {
    sessionLossUsd: 0,
    consecutiveQuoteFailures: 0,
    inventoryUsd: {},
    routeFamilyFailures: {},
    cooldownUntilMs: 0,
  };
}

// ── checkStrategyRisk ─────────────────────────────────────────────────────────

describe('checkStrategyRisk', () => {
  it('allows a clean trade in a fresh session', () => {
    const result = checkStrategyRisk(DECISION, freshState(), DEFAULT_RISK_LIMITS);
    expect(result.verdict).toBe('allow');
  });

  it('rejects during cooldown window', () => {
    const state: StrategyRiskState = {
      ...freshState(),
      cooldownUntilMs: Date.now() + 60_000,
    };
    const result = checkStrategyRisk(DECISION, state, DEFAULT_RISK_LIMITS, Date.now());
    expect(result.verdict).toBe('reject');
    expect(result.reason).toBe('cooldown_active');
  });

  it('allows after cooldown expires', () => {
    const state: StrategyRiskState = {
      ...freshState(),
      cooldownUntilMs: Date.now() - 1,
    };
    const result = checkStrategyRisk(DECISION, state, DEFAULT_RISK_LIMITS, Date.now());
    expect(result.verdict).toBe('allow');
  });

  it('halts when session loss exceeds limit', () => {
    const state: StrategyRiskState = {
      ...freshState(),
      sessionLossUsd: DEFAULT_RISK_LIMITS.maxSessionLossUsd,
    };
    const result = checkStrategyRisk(DECISION, state, DEFAULT_RISK_LIMITS);
    expect(result.verdict).toBe('halt');
    expect(result.reason).toBe('max_session_loss');
  });

  it('halts on consecutive quote failure streak', () => {
    const state: StrategyRiskState = {
      ...freshState(),
      consecutiveQuoteFailures: DEFAULT_RISK_LIMITS.maxConsecutiveQuoteFailures,
    };
    const result = checkStrategyRisk(DECISION, state, DEFAULT_RISK_LIMITS);
    expect(result.verdict).toBe('halt');
    expect(result.reason).toBe('quote_failure_streak');
  });

  it('rejects when route family has too many failures', () => {
    const state: StrategyRiskState = {
      ...freshState(),
      routeFamilyFailures: {
        'SOL/USDC': DEFAULT_RISK_LIMITS.maxRouteFamilyFailures,
      },
    };
    const result = checkStrategyRisk(DECISION, state, DEFAULT_RISK_LIMITS);
    expect(result.verdict).toBe('reject');
    expect(result.reason).toBe('route_family_cooldown');
  });

  it('rejects when inventory exceeds cap for input mint', () => {
    const state: StrategyRiskState = {
      ...freshState(),
      inventoryUsd: {
        [DECISION.inputMint]: DEFAULT_RISK_LIMITS.maxInventoryUsdPerAsset,
      },
    };
    const result = checkStrategyRisk(DECISION, state, DEFAULT_RISK_LIMITS);
    expect(result.verdict).toBe('reject');
    expect(result.reason).toBe('max_inventory');
  });

  it('uses custom limits when provided', () => {
    const strictLimits = { ...DEFAULT_RISK_LIMITS, maxSessionLossUsd: 5 };
    const state: StrategyRiskState = { ...freshState(), sessionLossUsd: 5 };
    const result = checkStrategyRisk(DECISION, state, strictLimits);
    expect(result.verdict).toBe('halt');
  });
});

// ── recordExecutionOutcome ────────────────────────────────────────────────────

describe('recordExecutionOutcome', () => {
  it('does not increment failure count on success', () => {
    const next = recordExecutionOutcome(
      freshState(),
      DECISION,
      1,
      true,
      DEFAULT_RISK_LIMITS,
    );
    expect(next.consecutiveQuoteFailures).toBe(0);
  });

  it('resets consecutive failures to zero after a success', () => {
    const state: StrategyRiskState = {
      ...freshState(),
      consecutiveQuoteFailures: 3,
    };
    const next = recordExecutionOutcome(state, DECISION, 1, true, DEFAULT_RISK_LIMITS);
    expect(next.consecutiveQuoteFailures).toBe(0);
  });

  it('accumulates session loss on negative PnL', () => {
    const next = recordExecutionOutcome(
      freshState(),
      DECISION,
      -5,
      true,
      DEFAULT_RISK_LIMITS,
    );
    expect(next.sessionLossUsd).toBeCloseTo(5);
  });

  it('does not accumulate session loss on profitable trade', () => {
    const next = recordExecutionOutcome(
      freshState(),
      DECISION,
      2,
      true,
      DEFAULT_RISK_LIMITS,
    );
    expect(next.sessionLossUsd).toBe(0);
  });

  it('increments consecutive failures on failed execution', () => {
    const next = recordExecutionOutcome(
      freshState(),
      DECISION,
      0,
      false,
      DEFAULT_RISK_LIMITS,
    );
    expect(next.consecutiveQuoteFailures).toBe(1);
  });

  it('triggers cooldown after maxConsecutiveQuoteFailures failures', () => {
    const state: StrategyRiskState = {
      ...freshState(),
      consecutiveQuoteFailures: DEFAULT_RISK_LIMITS.maxConsecutiveQuoteFailures - 1,
    };
    const now = Date.now();
    const next = recordExecutionOutcome(
      state,
      DECISION,
      0,
      false,
      DEFAULT_RISK_LIMITS,
      now,
    );
    expect(next.cooldownUntilMs).toBeGreaterThan(now);
  });

  it('accumulates route family failures on failed execution', () => {
    const next = recordExecutionOutcome(
      freshState(),
      DECISION,
      0,
      false,
      DEFAULT_RISK_LIMITS,
    );
    expect(next.routeFamilyFailures[DECISION.pairLabel]).toBe(1);
  });

  it('full session: loss accumulates until halt gate trips', () => {
    let state = freshState();
    const lossPerTrade = 10;
    const maxTrades = Math.ceil(DEFAULT_RISK_LIMITS.maxSessionLossUsd / lossPerTrade) + 1;
    let haltTripped = false;

    for (let i = 0; i < maxTrades; i++) {
      const check = checkStrategyRisk(DECISION, state, DEFAULT_RISK_LIMITS);
      if (check.verdict === 'halt') {
        haltTripped = true;
        break;
      }
      state = recordExecutionOutcome(
        state,
        DECISION,
        -lossPerTrade,
        true,
        DEFAULT_RISK_LIMITS,
      );
    }

    expect(haltTripped).toBe(true);
  });
});
