import { getAnchorPrice } from '@/lib/pricing/anchorCache';
import { SOL_MINT, tokenMeta, USDC_MINT } from '@/lib/terminal/tokens';
import type { SolanaCluster } from './network';

export type PaperSide = 'buy' | 'sell';

export interface PaperFill {
  id: string;
  cluster: SolanaCluster;
  side: PaperSide;
  tokenIn: string;
  tokenOut: string;
  amountIn: number;
  amountOut: number;
  priceUsd: number;
  slippageBps: number;
  pnlUsd: number;
  timestampMs: number;
  source: 'manual' | 'signal' | 'stream';
}

export interface PaperPortfolio {
  cluster: SolanaCluster;
  balances: Record<string, number>;
  costBasisUsd: Record<string, number>;
  fills: PaperFill[];
  realizedPnlUsd: number;
  startedAtMs: number;
}

const MAX_FILLS = 200;
const DEFAULT_SLIPPAGE_BPS = 30;

function storageKey(cluster: SolanaCluster): string {
  return `solarb_paper_${cluster}`;
}

function defaultBalances(cluster: SolanaCluster): Record<string, number> {
  if (cluster === 'devnet') {
    return {
      [SOL_MINT]: 10,
      [USDC_MINT]: 1_000,
    };
  }
  return {
    [SOL_MINT]: 5,
    [USDC_MINT]: 500,
  };
}

export function createPortfolio(cluster: SolanaCluster): PaperPortfolio {
  return {
    cluster,
    balances: defaultBalances(cluster),
    costBasisUsd: {},
    fills: [],
    realizedPnlUsd: 0,
    startedAtMs: Date.now(),
  };
}

export function loadPortfolio(cluster: SolanaCluster): PaperPortfolio {
  if (typeof window === 'undefined') return createPortfolio(cluster);
  try {
    const raw = localStorage.getItem(storageKey(cluster));
    if (!raw) return createPortfolio(cluster);
    const parsed = JSON.parse(raw) as PaperPortfolio;
    if (parsed.cluster !== cluster || !parsed.balances) return createPortfolio(cluster);
    return {
      ...createPortfolio(cluster),
      ...parsed,
      fills: (parsed.fills ?? []).slice(0, MAX_FILLS),
    };
  } catch {
    return createPortfolio(cluster);
  }
}

export function savePortfolio(portfolio: PaperPortfolio): void {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(storageKey(portfolio.cluster), JSON.stringify(portfolio));
  } catch {
    /* ignore quota */
  }
}

function priceForMint(mint: string, prices: Record<string, number>): number {
  if (prices[mint] && prices[mint] > 0) return prices[mint];
  const anchor = getAnchorPrice(mint);
  if (anchor && anchor > 0) return anchor;
  return tokenMeta(mint)?.refPrice ?? 0;
}

function applySlippage(amount: number, bps: number, adverse: boolean): number {
  const factor = adverse ? 1 - bps / 10_000 : 1 + bps / 10_000;
  return amount * factor;
}

export interface PaperSwapInput {
  cluster: SolanaCluster;
  portfolio: PaperPortfolio;
  tokenIn: string;
  tokenOut: string;
  amountIn: number;
  prices: Record<string, number>;
  slippageBps?: number;
  source?: PaperFill['source'];
}

export interface PaperSwapResult {
  ok: boolean;
  error?: string;
  portfolio: PaperPortfolio;
  fill?: PaperFill;
}

/** Simulate a swap against virtual balances using live/reference USD prices. */
export function executePaperSwap(input: PaperSwapInput): PaperSwapResult {
  const {
    cluster,
    portfolio,
    tokenIn,
    tokenOut,
    amountIn,
    prices,
    slippageBps = DEFAULT_SLIPPAGE_BPS,
    source = 'manual',
  } = input;

  if (amountIn <= 0) {
    return { ok: false, error: 'Amount must be positive', portfolio };
  }

  const balIn = portfolio.balances[tokenIn] ?? 0;
  if (balIn < amountIn) {
    return { ok: false, error: 'Insufficient paper balance', portfolio };
  }

  const priceIn = priceForMint(tokenIn, prices);
  const priceOut = priceForMint(tokenOut, prices);
  if (priceIn <= 0 || priceOut <= 0) {
    return { ok: false, error: 'Price unavailable for pair', portfolio };
  }

  const usdIn = amountIn * priceIn;
  let amountOut = usdIn / priceOut;
  amountOut = applySlippage(amountOut, slippageBps, true);

  const balances = { ...portfolio.balances };
  balances[tokenIn] = balIn - amountIn;
  balances[tokenOut] = (balances[tokenOut] ?? 0) + amountOut;

  const costBasisUsd = { ...portfolio.costBasisUsd };
  let realizedPnl = portfolio.realizedPnlUsd;

  if (tokenIn === SOL_MINT || tokenMeta(tokenIn)) {
    const prevCost = costBasisUsd[tokenOut] ?? priceOut;
    if (tokenOut !== USDC_MINT) {
      costBasisUsd[tokenOut] = prevCost;
    }
  }

  if (tokenOut === USDC_MINT || tokenIn !== USDC_MINT) {
    const basis = costBasisUsd[tokenIn] ?? priceIn;
    if (tokenIn !== SOL_MINT && tokenOut === USDC_MINT) {
      realizedPnl += usdIn - amountIn * basis;
    }
  }

  const fill: PaperFill = {
    id: `paper_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    cluster,
    side: tokenIn === SOL_MINT || tokenIn.startsWith('So1111') ? 'buy' : 'sell',
    tokenIn,
    tokenOut,
    amountIn,
    amountOut,
    priceUsd: priceOut,
    slippageBps,
    pnlUsd: amountOut * priceOut - usdIn,
    timestampMs: Date.now(),
    source,
  };

  const next: PaperPortfolio = {
    ...portfolio,
    balances,
    costBasisUsd,
    realizedPnlUsd: realizedPnl,
    fills: [fill, ...portfolio.fills].slice(0, MAX_FILLS),
  };

  savePortfolio(next);
  return { ok: true, portfolio: next, fill };
}

export function portfolioValueUsd(
  portfolio: PaperPortfolio,
  prices: Record<string, number>,
): number {
  let total = 0;
  for (const [mint, amount] of Object.entries(portfolio.balances)) {
    if (amount <= 0) continue;
    total += amount * priceForMint(mint, prices);
  }
  return total;
}

export function resetPortfolio(cluster: SolanaCluster): PaperPortfolio {
  const fresh = createPortfolio(cluster);
  savePortfolio(fresh);
  return fresh;
}
