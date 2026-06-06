/**
 * Integration tests for OpportunityDetector using pre-recorded account fixtures.
 * Connection is mocked — no mainnet traffic.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { buildOrcaPoolAccount, SOL_MINT, USDC_MINT } from './fixtures/orca-pool-account.js';
import { sqrtPriceX64ToUiPrice } from '../src/orca/whirlpool-monitor.js';
import type { ArbOpportunity } from '../src/detector/types.js';

// ─── mock @solana/web3.js ─────────────────────────────────────────────────────

vi.mock('@solana/web3.js', async (importActual) => {
  // Keep PublicKey working for fixture construction
  const actual = await importActual<typeof import('@solana/web3.js')>();
  return {
    ...actual,
    Connection: vi.fn(() => ({
      onAccountChange: vi.fn(() => 42),
      removeAccountChangeListener: vi.fn(() => Promise.resolve()),
      getBalance: vi.fn(() => Promise.resolve(5_000_000_000)),
      simulateTransaction: vi.fn(() =>
        Promise.resolve({ value: { err: null, unitsConsumed: 120_000 } }),
      ),
      sendRawTransaction: vi.fn(() => Promise.resolve('fake_signature_abc')),
      confirmTransaction: vi.fn(() => Promise.resolve({ value: { err: null } })),
      getRecentPrioritizationFees: vi.fn(() => Promise.resolve([{ prioritizationFee: 5_000 }])),
      getAddressLookupTable: vi.fn(() => Promise.resolve({ value: null })),
    })),
  };
});

// ─── fixture decoding ─────────────────────────────────────────────────────────

describe('Orca pool account fixture decoding', () => {
  it('decodes sqrtPriceX64 and recovers ~170 USDC per SOL', () => {
    const data = buildOrcaPoolAccount(170);

    // Read u128 LE from offset 65
    let sqrtPriceX64 = 0n;
    for (let i = 0; i < 16; i++) {
      sqrtPriceX64 |= BigInt(data[65 + i]) << BigInt(8 * i);
    }

    const price = sqrtPriceX64ToUiPrice(sqrtPriceX64, 9, 6);
    expect(price).toBeCloseTo(170, 0); // within ±1 USDC
  });

  it('tokenMintA at offset 101 matches SOL mint', () => {
    const { PublicKey } = await import('@solana/web3.js');
    const data = buildOrcaPoolAccount();
    const mintA = new PublicKey(data.subarray(101, 133)).toBase58();
    expect(mintA).toBe(SOL_MINT);
  });

  it('tokenMintB at offset 181 matches USDC mint', async () => {
    const { PublicKey } = await import('@solana/web3.js');
    const data = buildOrcaPoolAccount();
    const mintB = new PublicKey(data.subarray(181, 213)).toBase58();
    expect(mintB).toBe(USDC_MINT);
  });

  it('produces different sqrtPriceX64 for different prices', () => {
    const buf100 = buildOrcaPoolAccount(100);
    const buf200 = buildOrcaPoolAccount(200);
    let p100 = 0n, p200 = 0n;
    for (let i = 0; i < 16; i++) {
      p100 |= BigInt(buf100[65 + i]) << BigInt(8 * i);
      p200 |= BigInt(buf200[65 + i]) << BigInt(8 * i);
    }
    expect(p200).toBeGreaterThan(p100);
  });
});

// ─── OpportunityDetector ──────────────────────────────────────────────────────

describe('OpportunityDetector', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('emits opportunity when Orca price diverges from Jupiter reference', async () => {
    const { OpportunityDetector } = await import('../src/detector/opportunity-detector.js');

    const detector = new OpportunityDetector('https://api.mainnet-beta.solana.com', {
      minProfitLamports: 0,
      solPriceUsd: 170,
      priorityFeeLamports: 50_000,
      maxPriceStaleMs: 60_000,
      swapFeeBps: 30,
    });

    // Manually inject feeds to simulate account change + Jupiter poll
    const pairKey = `${SOL_MINT}:${USDC_MINT}`;
    // Access internal feeds map via type assertion for testing
    const feeds = (detector as unknown as Record<string, unknown>)['feeds'] as Map<string, Map<string, { dexName: string; price: number; lastUpdatedMs: number }>>;
    feeds.set(pairKey, new Map([
      ['orca',    { dexName: 'orca',    price: 170,   lastUpdatedMs: Date.now() }],
      ['jupiter', { dexName: 'jupiter', price: 171.5, lastUpdatedMs: Date.now() }],
    ]));

    const opportunities: ArbOpportunity[] = [];
    detector.on('opportunity', (o: ArbOpportunity) => opportunities.push(o));

    // Trigger evaluation via the private method (white-box test)
    const evaluate = (detector as unknown as Record<string, unknown>)['evaluatePair'] as (a: string, b: string, k: string) => void;
    evaluate.call(detector, SOL_MINT, USDC_MINT, pairKey);

    expect(opportunities).toHaveLength(1);
    const opp = opportunities[0];
    expect(opp.dex1).toBe('orca');
    expect(opp.dex2).toBe('jupiter');
    expect(opp.profitUsd).toBeGreaterThan(0);
    expect(opp.grossSpreadBps).toBeGreaterThan(0);
    expect(opp.route).toEqual([SOL_MINT, USDC_MINT, SOL_MINT]);
  });

  it('does NOT emit when spread is profitable but below minProfitLamports', async () => {
    const { OpportunityDetector } = await import('../src/detector/opportunity-detector.js');

    const detector = new OpportunityDetector('https://api.mainnet-beta.solana.com', {
      minProfitLamports: 1_000_000_000, // 1 SOL minimum — impossible to meet
      solPriceUsd: 170,
      priorityFeeLamports: 50_000,
      maxPriceStaleMs: 60_000,
      swapFeeBps: 30,
    });

    const pairKey = `${SOL_MINT}:${USDC_MINT}`;
    const feeds = (detector as unknown as Record<string, unknown>)['feeds'] as Map<string, Map<string, { dexName: string; price: number; lastUpdatedMs: number }>>;
    feeds.set(pairKey, new Map([
      ['orca',    { dexName: 'orca',    price: 170,   lastUpdatedMs: Date.now() }],
      ['jupiter', { dexName: 'jupiter', price: 171.5, lastUpdatedMs: Date.now() }],
    ]));

    const opportunities: ArbOpportunity[] = [];
    detector.on('opportunity', (o: ArbOpportunity) => opportunities.push(o));

    const evaluate = (detector as unknown as Record<string, unknown>)['evaluatePair'] as (a: string, b: string, k: string) => void;
    evaluate.call(detector, SOL_MINT, USDC_MINT, pairKey);

    expect(opportunities).toHaveLength(0);
  });

  it('does NOT emit when only one feed is active (need ≥2)', async () => {
    const { OpportunityDetector } = await import('../src/detector/opportunity-detector.js');

    const detector = new OpportunityDetector('https://api.mainnet-beta.solana.com', {
      minProfitLamports: 0,
      solPriceUsd: 170,
    });

    const pairKey = `${SOL_MINT}:${USDC_MINT}`;
    const feeds = (detector as unknown as Record<string, unknown>)['feeds'] as Map<string, Map<string, { dexName: string; price: number; lastUpdatedMs: number }>>;
    feeds.set(pairKey, new Map([
      ['orca', { dexName: 'orca', price: 170, lastUpdatedMs: Date.now() }],
    ]));

    const opportunities: ArbOpportunity[] = [];
    detector.on('opportunity', (o: ArbOpportunity) => opportunities.push(o));

    const evaluate = (detector as unknown as Record<string, unknown>)['evaluatePair'] as (a: string, b: string, k: string) => void;
    evaluate.call(detector, SOL_MINT, USDC_MINT, pairKey);

    expect(opportunities).toHaveLength(0);
  });

  it('opportunity object has all required fields', async () => {
    const { OpportunityDetector } = await import('../src/detector/opportunity-detector.js');

    const detector = new OpportunityDetector('https://api.mainnet-beta.solana.com', {
      minProfitLamports: 0,
      solPriceUsd: 170,
      maxPriceStaleMs: 60_000,
    });

    const pairKey = `${SOL_MINT}:${USDC_MINT}`;
    const feeds = (detector as unknown as Record<string, unknown>)['feeds'] as Map<string, Map<string, { dexName: string; price: number; lastUpdatedMs: number }>>;
    feeds.set(pairKey, new Map([
      ['orca',    { dexName: 'orca',    price: 168, lastUpdatedMs: Date.now() }],
      ['jupiter', { dexName: 'jupiter', price: 172, lastUpdatedMs: Date.now() }],
    ]));

    let captured: ArbOpportunity | null = null;
    detector.on('opportunity', (o: ArbOpportunity) => { captured = o; });

    const evaluate = (detector as unknown as Record<string, unknown>)['evaluatePair'] as (a: string, b: string, k: string) => void;
    evaluate.call(detector, SOL_MINT, USDC_MINT, pairKey);

    expect(captured).not.toBeNull();
    const o = captured!;
    // Verify all required fields per ArbOpportunity spec
    expect(typeof o.tokenIn).toBe('string');
    expect(typeof o.tokenOut).toBe('string');
    expect(typeof o.dex1).toBe('string');
    expect(typeof o.dex2).toBe('string');
    expect(typeof o.profitUsd).toBe('number');
    expect(typeof o.profitLamports).toBe('number');
    expect(typeof o.profitToRiskRatio).toBe('number');
    expect(typeof o.grossSpreadBps).toBe('number');
    expect(Array.isArray(o.route)).toBe(true);
    expect(typeof o.detectedAtMs).toBe('number');
    expect(o.feesUsd).toBeDefined();
  });
});
