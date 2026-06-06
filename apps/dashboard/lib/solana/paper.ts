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
  /** Weighted-average cost basis in USD per token unit. */
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

function defaultCostBasis(): Record<string, number> {
  return {
    [SOL_MINT]: 0,
    [USDC_MINT]: 1,
  };
}

export function createPortfolio(cluster: SolanaCluster): PaperPortfolio {
  return {
    cluster,
    balances: defaultBalances(cluster),
    costBasisUsd: defaultCostBasis(),
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
      costBasisUsd: { ...defaultCostBasis(), ...(parsed.costBasisUsd ?? {}) },
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

function isQuoteMint(mint: string): boolean {
  return mint === USDC_MINT;
}

function isPositionMint(mint: string): boolean {
  return !isQuoteMint(mint);
}

function inferSide(tokenIn: string, tokenOut: string): PaperSide {
  if (isQuoteMint(tokenIn) && isPositionMint(tokenOut)) return 'buy';
  if (isPositionMint(tokenIn) && isQuoteMint(tokenOut)) return 'sell';
  if (tokenIn === SOL_MINT && isPositionMint(tokenOut)) return 'buy';
  if (isPositionMint(tokenIn) && tokenOut === SOL_MINT) return 'sell';
  return 'buy';
}

/** Update weighted-average cost basis when acquiring a position token. */
function recordAcquisition(
  costBasisUsd: Record<string, number>,
  mint: string,
  prevQty: number,
  acquiredQty: number,
  usdSpent: number,
): void {
  if (acquiredQty <= 0 || usdSpent <= 0) return;
  const prevBasis = costBasisUsd[mint] ?? 0;
  const newQty = prevQty + acquiredQty;
  costBasisUsd[mint] =
    newQty > 0 ? (prevQty * prevBasis + usdSpent) / newQty : prevBasis;
}

/** Realized PnL when disposing a position token at market USD price. */
function recordDisposal(
  costBasisUsd: Record<string, number>,
  mint: string,
  amountSold: number,
  marketUsdPerUnit: number,
): number {
  if (amountSold <= 0) return 0;
  const basis = costBasisUsd[mint] ?? marketUsdPerUnit;
  return amountSold * (marketUsdPerUnit - basis);
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

  const prevOutQty = portfolio.balances[tokenOut] ?? 0;
  const costBasisUsd = { ...portfolio.costBasisUsd };
  let realizedPnl = portfolio.realizedPnlUsd;
  let fillPnlUsd = 0;

  if (isPositionMint(tokenIn)) {
    const realized = recordDisposal(costBasisUsd, tokenIn, amountIn, priceIn);
    realizedPnl += realized;
    fillPnlUsd += realized;
  }

  if (isPositionMint(tokenOut)) {
    recordAcquisition(costBasisUsd, tokenOut, prevOutQty, amountOut, usdIn);
    const markValue = amountOut * priceOut;
    fillPnlUsd += markValue - usdIn;
  } else if (isQuoteMint(tokenOut)) {
    fillPnlUsd = realizedPnl - portfolio.realizedPnlUsd;
  }

  const balances = { ...portfolio.balances };
  balances[tokenIn] = balIn - amountIn;
  balances[tokenOut] = prevOutQty + amountOut;

  const fill: PaperFill = {
    id: `paper_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    cluster,
    side: inferSide(tokenIn, tokenOut),
    tokenIn,
    tokenOut,
    amountIn,
    amountOut,
    priceUsd: priceOut,
    slippageBps,
    pnlUsd: fillPnlUsd,
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

export function unrealizedPnlUsd(
  portfolio: PaperPortfolio,
  prices: Record<string, number>,
): number {
  let unrealized = 0;
  for (const [mint, amount] of Object.entries(portfolio.balances)) {
    if (amount <= 0 || !isPositionMint(mint)) continue;
    const price = priceForMint(mint, prices);
    const basis = portfolio.costBasisUsd[mint] ?? price;
    unrealized += amount * (price - basis);
  }
  return unrealized;
}

export function resetPortfolio(cluster: SolanaCluster): PaperPortfolio {
  const fresh = createPortfolio(cluster);
  savePortfolio(fresh);
  return fresh;
}
