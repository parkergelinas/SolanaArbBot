import { create } from 'zustand';

import {
  executePaperSwap,
  loadPortfolio,
  portfolioValueUsd,
  resetPortfolio,
  type PaperFill,
  type PaperPortfolio,
  type PaperSide,
} from '@/lib/solana/paper';
import { SOL_MINT, USDC_MINT } from '@/lib/terminal/tokens';
import type { SolanaCluster } from '@/lib/solana/network';

interface PaperState {
  enabled: boolean;
  autoTrade: boolean;
  portfolio: PaperPortfolio | null;
  lastFill: PaperFill | null;
  error: string | null;

  initForCluster: (cluster: SolanaCluster) => void;
  setEnabled: (v: boolean) => void;
  setAutoTrade: (v: boolean) => void;
  swap: (
    tokenIn: string,
    tokenOut: string,
    amountIn: number,
    prices: Record<string, number>,
    source?: PaperFill['source'],
  ) => boolean;
  quickTrade: (
    mint: string,
    side: PaperSide,
    solAmount: number,
    prices: Record<string, number>,
  ) => boolean;
  reset: (cluster: SolanaCluster) => void;
  equityUsd: (prices: Record<string, number>) => number;
}

export const usePaperStore = create<PaperState>((set, get) => ({
  enabled: true,
  autoTrade: false,
  portfolio: null,
  lastFill: null,
  error: null,

  initForCluster: (cluster) => {
    const { portfolio } = get();
    if (portfolio?.cluster === cluster) return;
    set({ portfolio: loadPortfolio(cluster), error: null });
  },

  setEnabled: (v) => set({ enabled: v }),

  setAutoTrade: (v) => set({ autoTrade: v }),

  swap: (tokenIn, tokenOut, amountIn, prices, source) => {
    const { portfolio, enabled } = get();
    if (!enabled || !portfolio) return false;

    const result = executePaperSwap({
      cluster: portfolio.cluster,
      portfolio,
      tokenIn,
      tokenOut,
      amountIn,
      prices,
      source,
    });

    if (!result.ok) {
      set({ error: result.error ?? 'Swap failed' });
      return false;
    }

    set({
      portfolio: result.portfolio,
      lastFill: result.fill ?? null,
      error: null,
    });
    return true;
  },

  quickTrade: (mint, side, solAmount, prices) => {
    const tokenIn = side === 'buy' ? SOL_MINT : mint;
    const tokenOut = side === 'buy' ? mint : SOL_MINT;
    const amountIn =
      side === 'buy'
        ? solAmount
        : (() => {
            const priceMint = prices[mint] ?? 0;
            const priceSol = prices[SOL_MINT] ?? 0;
            if (priceMint <= 0 || priceSol <= 0) return 0;
            return (solAmount * priceSol) / priceMint;
          })();

    return get().swap(tokenIn, tokenOut, amountIn, prices, 'manual');
  },

  reset: (cluster) => {
    const fresh = resetPortfolio(cluster);
    set({ portfolio: fresh, lastFill: null, error: null });
  },

  equityUsd: (prices) => {
    const { portfolio } = get();
    if (!portfolio) return 0;
    return portfolioValueUsd(portfolio, prices);
  },
}));
