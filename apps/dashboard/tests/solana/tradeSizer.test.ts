import { describe, expect, it } from 'vitest';
import {
  computeFrontendTradeSize,
  type FrontendSizeResult,
} from '@/lib/solana/tradeSizer';

// ── Constants mirrored from tradeSizer.ts ────────────────────────────────────
const NO_JITO_MAX_SOL = 0.42;
const MIN_SOL = 0.10;

describe('computeFrontendTradeSize', () => {
  it('returns at least MIN_SOL on very narrow spread', () => {
    const result = computeFrontendTradeSize(1, 150);
    expect(result.solAmount).toBeGreaterThanOrEqual(MIN_SOL);
  });

  it('caps at NO_JITO_MAX_SOL ceiling', () => {
    const result = computeFrontendTradeSize(200, 150, {
      liquidityUsd: 10_000_000,
      recentFailureRate: 0,
    });
    expect(result.solAmount).toBeLessThanOrEqual(NO_JITO_MAX_SOL);
  });

  it('narrow spread keeps ceiling at NO_JITO_MAX_SOL (mevSafeSol >> ceiling)', () => {
    // 5 bps at $150: mevSafeSol = 0.50 / (0.0005 * 150) = 66.67 → capped at 0.42
    const result = computeFrontendTradeSize(5, 150);
    expect(result.mevSafeSol).toBeGreaterThan(1);
    expect(result.solAmount).toBeLessThanOrEqual(NO_JITO_MAX_SOL);
    expect(result.binding).toBe('no_jito_cap');
  });

  it('wide spread at high SOL price makes mevSafeSol the tighter bound', () => {
    // 40 bps at $500: mevSafeSol = 0.50 / (0.004 * 500) = 0.25 < 0.42
    const result = computeFrontendTradeSize(40, 500, {
      recentFailureRate: 0,
      liquidityUsd: 10_000_000,
    });
    expect(result.mevSafeSol).toBeCloseTo(0.25, 2);
    expect(result.binding).toBe('mev_threshold');
    expect(result.solAmount).toBeLessThanOrEqual(0.25);
  });

  it('higher failure rate produces smaller or equal trade size', () => {
    const clean = computeFrontendTradeSize(40, 150, { recentFailureRate: 0 });
    const failing = computeFrontendTradeSize(40, 150, { recentFailureRate: 0.8 });
    expect(failing.solAmount).toBeLessThanOrEqual(clean.solAmount);
  });

  it('thin liquidity reduces trade size vs deep pool', () => {
    const thin = computeFrontendTradeSize(40, 150, { liquidityUsd: 10_000 });
    const deep = computeFrontendTradeSize(40, 150, { liquidityUsd: 5_000_000 });
    expect(thin.solAmount).toBeLessThanOrEqual(deep.solAmount);
  });

  it('result contains all required fields', () => {
    const result: FrontendSizeResult = computeFrontendTradeSize(40, 150);
    expect(typeof result.solAmount).toBe('number');
    expect(typeof result.mevSafeSol).toBe('number');
    expect(typeof result.label).toBe('string');
    expect(['mev_threshold', 'no_jito_cap', 'min_clamp']).toContain(result.binding);
  });

  it('label contains spread and SOL amount', () => {
    const result = computeFrontendTradeSize(40, 150);
    expect(result.label).toContain('40bps');
    expect(result.label).toContain('SOL');
  });

  it('zero sol price falls back safely (no divide-by-zero)', () => {
    const result = computeFrontendTradeSize(40, 0);
    expect(result.solAmount).toBeGreaterThanOrEqual(MIN_SOL);
    expect(result.solAmount).toBeLessThanOrEqual(NO_JITO_MAX_SOL);
    // When price is 0 the fallback is NO_JITO_MAX_SOL, so binding must not be mev_threshold
    expect(result.binding).not.toBe('mev_threshold');
  });

  it('min_clamp binding fires when all factors produce sub-minimum result', () => {
    // Extreme failure rate + thin liquidity + narrow spread → raw below MIN_SOL
    const result = computeFrontendTradeSize(1, 150, {
      recentFailureRate: 1.0,  // → factor 0.20
      liquidityUsd: 1_000,     // → liqFactor 0.50
    });
    // Even with clamp to MIN_SOL
    expect(result.solAmount).toBeGreaterThanOrEqual(MIN_SOL);
    expect(result.binding).toBe('min_clamp');
  });

  it('mev_safe_sol equation: output matches manual calculation', () => {
    // 80 bps at $200: mevSafeSol = 0.50 / (0.008 * 200) = 0.50 / 1.6 = 0.3125
    const result = computeFrontendTradeSize(80, 200, {
      recentFailureRate: 0,
      liquidityUsd: 2_000_000,
    });
    expect(result.mevSafeSol).toBeCloseTo(0.3125, 3);
  });
});
