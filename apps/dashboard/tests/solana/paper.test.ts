import { describe, expect, it } from 'vitest';

import {
  createPortfolio,
  executePaperSwap,
  portfolioValueUsd,
  unrealizedPnlUsd,
} from '@/lib/solana/paper';
import { SOL_MINT, USDC_MINT } from '@/lib/terminal/tokens';

const BONK = 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263';

describe('paper trading simulation', () => {
  it('executes a SOL → alt buy with slippage', () => {
    const portfolio = createPortfolio('devnet');
    const prices = {
      [SOL_MINT]: 100,
      [USDC_MINT]: 1,
      [BONK]: 0.00002,
    };

    const result = executePaperSwap({
      cluster: 'devnet',
      portfolio,
      tokenIn: SOL_MINT,
      tokenOut: BONK,
      amountIn: 1,
      prices,
    });

    expect(result.ok).toBe(true);
    expect(result.portfolio!.balances[SOL_MINT]).toBe(9);
    expect(result.fill?.side).toBe('buy');
    expect(result.portfolio!.costBasisUsd[BONK]).toBeGreaterThan(0);
  });

  it('rejects swap when balance insufficient', () => {
    const portfolio = createPortfolio('mainnet-beta');
    const result = executePaperSwap({
      cluster: 'mainnet-beta',
      portfolio,
      tokenIn: SOL_MINT,
      tokenOut: USDC_MINT,
      amountIn: 100,
      prices: { [SOL_MINT]: 100, [USDC_MINT]: 1 },
    });
    expect(result.ok).toBe(false);
  });

  it('computes portfolio equity', () => {
    const portfolio = createPortfolio('devnet');
    const equity = portfolioValueUsd(portfolio, {
      [SOL_MINT]: 100,
      [USDC_MINT]: 1,
    });
    expect(equity).toBeGreaterThan(1000);
  });

  it('tracks realized PnL on sell with weighted cost basis', () => {
    const prices = {
      [SOL_MINT]: 100,
      [USDC_MINT]: 1,
      [BONK]: 0.00002,
    };

    const buy = executePaperSwap({
      cluster: 'devnet',
      portfolio: createPortfolio('devnet'),
      tokenIn: SOL_MINT,
      tokenOut: BONK,
      amountIn: 1,
      prices,
      slippageBps: 0,
    });
    expect(buy.ok).toBe(true);

    const higherPrices = { ...prices, [BONK]: 0.00004 };
    const bonkHeld = buy.portfolio!.balances[BONK] ?? 0;
    const sell = executePaperSwap({
      cluster: 'devnet',
      portfolio: buy.portfolio!,
      tokenIn: BONK,
      tokenOut: USDC_MINT,
      amountIn: bonkHeld,
      prices: higherPrices,
      slippageBps: 0,
    });

    expect(sell.ok).toBe(true);
    expect(sell.fill?.side).toBe('sell');
    expect(sell.portfolio!.realizedPnlUsd).toBeGreaterThan(0);
    expect(sell.fill?.pnlUsd).toBeGreaterThan(0);
  });

  it('reports unrealized PnL from cost basis', () => {
    const prices = {
      [SOL_MINT]: 100,
      [USDC_MINT]: 1,
      [BONK]: 0.00002,
    };

    const buy = executePaperSwap({
      cluster: 'devnet',
      portfolio: createPortfolio('devnet'),
      tokenIn: USDC_MINT,
      tokenOut: BONK,
      amountIn: 50,
      prices,
      slippageBps: 0,
    });

    const markUp = { ...prices, [BONK]: 0.00003 };
    const unrealized = unrealizedPnlUsd(buy.portfolio!, markUp);
    expect(unrealized).toBeGreaterThan(0);
  });
});
