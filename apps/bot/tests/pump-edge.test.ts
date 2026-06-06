import { describe, expect, it } from 'vitest';

import {
  buyPriceImpactBps,
  buyTokensForSol,
  decodeBondingCurveAccount,
  freshBondingCurve,
  graduationProgressPct,
  spotPriceSol,
} from '../src/pump/bonding-curve.js';
import { scorePumpEdge } from '../src/pump/edge-scorer.js';
import { INITIAL_REAL_TOKEN_RESERVES } from '../src/pump/constants.js';

describe('bonding curve math', () => {
  it('fresh curve has zero graduation progress', () => {
    const curve = freshBondingCurve('TestMint1111111111111111111111111111111');
    expect(graduationProgressPct(curve)).toBe(0);
    expect(curve.complete).toBe(false);
  });

  it('spot price is positive on fresh curve', () => {
    const curve = freshBondingCurve('TestMint1111111111111111111111111111111');
    expect(spotPriceSol(curve)).toBeGreaterThan(0);
  });

  it('buy increases graduation progress', () => {
    const curve = freshBondingCurve('TestMint1111111111111111111111111111111');
    const solIn = 1_000_000_000n;
    const tokens = buyTokensForSol(curve, solIn);
    expect(tokens).toBeGreaterThan(0n);

    const updated = {
      ...curve,
      virtualSolReserves: curve.virtualSolReserves + solIn,
      virtualTokenReserves: curve.virtualTokenReserves - tokens,
      realTokenReserves: curve.realTokenReserves - tokens,
      realSolReserves: curve.realSolReserves + solIn,
    };
    expect(graduationProgressPct(updated)).toBeGreaterThan(0);
  });

  it('price impact increases with trade size', () => {
    const curve = freshBondingCurve('TestMint1111111111111111111111111111111');
    const small = buyPriceImpactBps(curve, 100_000_000n);
    const large = buyPriceImpactBps(curve, 5_000_000_000n);
    expect(large).toBeGreaterThan(small);
  });

  it('decodes bonding curve account layout', () => {
    const buf = Buffer.alloc(64);
    buf.writeBigUInt64LE(1_073_000_000_000_000n, 8);
    buf.writeBigUInt64LE(30_000_000_000n, 16);
    buf.writeBigUInt64LE(INITIAL_REAL_TOKEN_RESERVES, 24);
    buf.writeBigUInt64LE(5_000_000_000n, 32);
    buf.writeBigUInt64LE(1_000_000_000_000_000n, 40);
    buf[48] = 0;

    const state = decodeBondingCurveAccount('mint123', buf);
    expect(state).not.toBeNull();
    expect(state!.realSolReserves).toBe(5_000_000_000n);
    expect(state!.complete).toBe(false);
  });
});

describe('pump edge scorer', () => {
  it('detects curve-jupiter divergence', () => {
    const curve = freshBondingCurve('TestMint1111111111111111111111111111111');
    const signal = scorePumpEdge(curve, 150, 0.00005, {
      minEdgeBps: 50,
      tradeSol: 0.5,
      minGraduationPct: 80,
      maxGraduationPct: 99,
      maxEarlyImpactBps: 300,
      minCurveJupiterDivergenceBps: 75,
    });
    expect(signal).not.toBeNull();
    expect(signal!.edgeBps).toBeGreaterThanOrEqual(50);
  });

  it('detects early curve entry', () => {
    const curve = freshBondingCurve('TestMint1111111111111111111111111111111');
    const signal = scorePumpEdge(curve, 150, undefined, {
      minEdgeBps: 20,
      tradeSol: 0.1,
      minGraduationPct: 80,
      maxGraduationPct: 99,
      maxEarlyImpactBps: 500,
      minCurveJupiterDivergenceBps: 10_000,
    });
    expect(signal).not.toBeNull();
    expect(signal!.signalType).toBe('early_curve_entry');
  });
});
