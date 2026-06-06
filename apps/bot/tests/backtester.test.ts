import { describe, expect, it } from 'vitest';

import { runBacktest } from '../src/backtest/backtester.js';
import type { BacktestQuoteRecord } from '../src/backtest/backtester.js';

describe('runBacktest', () => {
  it('applies execution economics beyond raw spread', () => {
    const SOL = 'So11111111111111111111111111111111111111112';
    const USDC = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';

    const records: BacktestQuoteRecord[] = [
      {
        inputDecimals: 9,
        state: {
          timestampMs: Date.now(),
          pricesUsd: { [SOL]: 100, [USDC]: 1 },
          decimals: { [SOL]: 9, [USDC]: 6 },
          quality: {},
          universe: [SOL, USDC],
          solPriceUsd: 100,
        },
        pair: {
          pairLabel: 'SOL/USDC',
          forward: {
            request: { inputMint: SOL, outputMint: USDC, amount: '1000000000' },
            response: { inAmount: '1000000000', outAmount: '150000000' },
            capturedAtMs: Date.now(),
          },
          reverse: {
            request: { inputMint: USDC, outputMint: SOL, amount: '150000000' },
            response: { inAmount: '150000000', outAmount: '1010000000' },
            capturedAtMs: Date.now(),
          },
        },
      },
    ];

    const result = runBacktest(records, {
      quoteStalenessMs: 300,
      latencyDriftMs: 150,
      landingSlippageBps: 15,
      missedFillRate: 0,
      priorityFeeLamports: 80_000,
      routeChangePenaltyBps: 8,
    });

    expect(result.opportunities).toBe(1);
    expect(result.totalSimulatedUsd).toBeLessThan(result.totalExpectedUsd);
  });
});
