import { describe, expect, it } from 'vitest';

import {
  createPortfolio,
  executePaperSwap,
  portfolioValueUsd,
} from '@/lib/solana/paper';
import { SOL_MINT, USDC_MINT } from '@/lib/terminal/tokens';

describe('paper trading simulation', () => {
  it('executes a SOL → alt buy with slippage', () => {
    const portfolio = createPortfolio('devnet');
    const prices = {
      [SOL_MINT]: 100,
      [USDC_MINT]: 1,
      DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263: 0.00002,
    };

    const result = executePaperSwap({
      cluster: 'devnet',
      portfolio,
      tokenIn: SOL_MINT,
      tokenOut: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263',
      amountIn: 1,
      prices,
    });

    expect(result.ok).toBe(true);
    expect(result.portfolio!.balances[SOL_MINT]).toBe(9);
    expect(result.fill?.side).toBe('buy');
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
});
