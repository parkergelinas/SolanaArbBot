/**
 * Unit tests for the MEV-aware dynamic trade sizer.
 *
 * Validates the core theorem: the MEV sandwich threshold equation produces
 * correct ceilings across representative (spread, price, jito) combinations,
 * and that secondary factors only move size downward from that ceiling.
 */

import { describe, it, expect } from 'vitest';
import { computeDynamicSize } from '../src/sizing/dynamic.js';

const BASE: Parameters<typeof computeDynamicSize>[0] = {
  baseAmountUi: 0.5,
  spreadBps: 36,
  liquidityUsd: 500_000,
  volatilityPct: 2,
  routeQualityScore: 0.8,
  recentFailureRate: 0,
  minAmountUi: 0.10,
  maxAmountUi: 1.0,
  solPriceUsd: 180,
  jitoActive: false,
  priorityFeeMicroLamports: 50_000,
};

// ── MEV ceiling equation ──────────────────────────────────────────────────────

describe('MEV threshold ceiling', () => {
  it('computes mev_safe_sol correctly: $0.50 / (36bps × $180) = 0.77 SOL', () => {
    const r = computeDynamicSize(BASE);
    // mev_safe = 0.50 / (0.0036 × 180) = 0.771...
    expect(r.mevSafeSol).toBeCloseTo(0.77, 1);
  });

  it('mev_safe_sol shrinks as spread widens (more extractable per SOL)', () => {
    const wide = computeDynamicSize({ ...BASE, spreadBps: 100 });
    const narrow = computeDynamicSize({ ...BASE, spreadBps: 36 });
    // At 100 bps: $0.50 / (0.01 × $180) = 0.28 SOL
    expect(wide.mevSafeSol).toBeLessThan(narrow.mevSafeSol);
    expect(wide.mevSafeSol).toBeCloseTo(0.28, 1);
  });

  it('without Jito at 36 bps the ceiling is the 0.42 SOL no-jito default (not 0.77 mev_safe)', () => {
    // mev_safe = 0.77, but no-jito default = 0.42 — the tighter bound wins
    const r = computeDynamicSize(BASE);
    expect(r.amountUi).toBeLessThanOrEqual(0.42);
  });

  it('at 100 bps without Jito, mev_threshold is tighter than no-jito default', () => {
    const r = computeDynamicSize({ ...BASE, spreadBps: 100 });
    // mev_safe = 0.28 < 0.42 → mev_threshold is binding
    expect(r.ceiling).toBe('mev_threshold');
    expect(r.amountUi).toBeLessThanOrEqual(0.28);
  });
});

// ── Jito protection ───────────────────────────────────────────────────────────

describe('Jito ceiling override', () => {
  it('with Jito active, ceiling is 0.80 SOL regardless of spread', () => {
    const narrow = computeDynamicSize({ ...BASE, jitoActive: true, spreadBps: 20 });
    const wide = computeDynamicSize({ ...BASE, jitoActive: true, spreadBps: 200 });
    // Both should be bounded by 0.80, not the MEV equation
    expect(narrow.ceiling).toBe('jito');
    expect(wide.ceiling).toBe('jito');
    expect(narrow.amountUi).toBeLessThanOrEqual(0.80);
    expect(wide.amountUi).toBeLessThanOrEqual(0.80);
  });

  it('Jito at 36 bps produces larger size than no-Jito at 36 bps', () => {
    const withJito = computeDynamicSize({ ...BASE, jitoActive: true });
    const noJito = computeDynamicSize({ ...BASE, jitoActive: false });
    expect(withJito.amountUi).toBeGreaterThan(noJito.amountUi);
  });
});

// ── Secondary factors only shrink ─────────────────────────────────────────────

describe('Secondary factors (all should reduce, never exceed ceiling)', () => {
  const jitoBase = { ...BASE, jitoActive: true }; // use Jito so ceiling = 0.80 SOL

  it('50% failure rate reduces size vs 0% failure rate', () => {
    const healthy = computeDynamicSize({ ...jitoBase, recentFailureRate: 0 });
    const degraded = computeDynamicSize({ ...jitoBase, recentFailureRate: 0.5 });
    expect(degraded.amountUi).toBeLessThan(healthy.amountUi);
  });

  it('thin liquidity (<$50k) reduces size vs deep liquidity (>$1M)', () => {
    const deep = computeDynamicSize({ ...jitoBase, liquidityUsd: 2_000_000 });
    const thin = computeDynamicSize({ ...jitoBase, liquidityUsd: 20_000 });
    expect(thin.amountUi).toBeLessThan(deep.amountUi);
  });

  it('high congestion (400k µ-lamports) reduces size vs low (10k)', () => {
    const quiet = computeDynamicSize({ ...jitoBase, priorityFeeMicroLamports: 10_000 });
    const busy = computeDynamicSize({ ...jitoBase, priorityFeeMicroLamports: 400_000 });
    expect(busy.amountUi).toBeLessThan(quiet.amountUi);
  });

  it('poor route quality (0.3) reduces size vs excellent (1.0)', () => {
    const good = computeDynamicSize({ ...jitoBase, routeQualityScore: 1.0 });
    const poor = computeDynamicSize({ ...jitoBase, routeQualityScore: 0.3 });
    expect(poor.amountUi).toBeLessThan(good.amountUi);
  });

  it('no factor combination pushes size above the Jito ceiling', () => {
    const aggressive = computeDynamicSize({
      ...jitoBase,
      spreadBps: 200,
      liquidityUsd: 10_000_000,
      routeQualityScore: 1.0,
      recentFailureRate: 0,
      priorityFeeMicroLamports: 1_000,
    });
    expect(aggressive.amountUi).toBeLessThanOrEqual(0.80);
  });
});

// ── Minimum clamp ─────────────────────────────────────────────────────────────

describe('Minimum size clamp', () => {
  it('worst-case inputs still produce at least minAmountUi', () => {
    const r = computeDynamicSize({
      ...BASE,
      spreadBps: 5,            // below break-even
      liquidityUsd: 1_000,     // extremely thin
      recentFailureRate: 1.0,  // all failures
      routeQualityScore: 0,
      priorityFeeMicroLamports: 500_000, // maximum congestion
    });
    expect(r.amountUi).toBeGreaterThanOrEqual(0.10);
    expect(r.ceiling).toBe('min_clamp');
  });
});

// ── Worked example: optimal size at recommended settings ─────────────────────

describe('Recommended config worked example', () => {
  it('no-Jito @ 36 bps → ~0.40 SOL', () => {
    // With all healthy factors and no-Jito: ceiling=0.42, factors ≈ 0.95+
    const r = computeDynamicSize({
      ...BASE,
      jitoActive: false,
      spreadBps: 36,
      solPriceUsd: 180,
      liquidityUsd: 1_000_000,
      recentFailureRate: 0,
      routeQualityScore: 0.9,
      priorityFeeMicroLamports: 20_000,
    });
    expect(r.amountUi).toBeGreaterThanOrEqual(0.35);
    expect(r.amountUi).toBeLessThanOrEqual(0.42);
  });

  it('Jito @ 36 bps → ~0.72–0.80 SOL', () => {
    const r = computeDynamicSize({
      ...BASE,
      jitoActive: true,
      spreadBps: 36,
      solPriceUsd: 180,
      liquidityUsd: 1_000_000,
      recentFailureRate: 0,
      routeQualityScore: 0.9,
      priorityFeeMicroLamports: 20_000,
    });
    expect(r.amountUi).toBeGreaterThanOrEqual(0.70);
    expect(r.amountUi).toBeLessThanOrEqual(0.80);
  });
});
