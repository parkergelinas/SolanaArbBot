/** Canonical watchlist mints for terminal demo + display. */
export const SOL_MINT = 'So11111111111111111111111111111111111111112';
export const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
export const USDT_MINT = 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB';

export interface WatchToken {
  mint: string;
  symbol: string;
  name: string;
  refPrice: number;
  decimals: number;
}

/** refPrice is bootstrap-only fallback — live USD comes from DexScreener via PriceBootstrap. */
export const WATCHLIST: WatchToken[] = [
  { mint: SOL_MINT, symbol: 'SOL', name: 'Solana', refPrice: 0, decimals: 9 },
  { mint: USDC_MINT, symbol: 'USDC', name: 'USD Coin', refPrice: 1.0, decimals: 6 },
  { mint: USDT_MINT, symbol: 'USDT', name: 'Tether', refPrice: 1.0, decimals: 6 },
  {
    mint: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263',
    symbol: 'BONK',
    name: 'Bonk',
    refPrice: 0,
    decimals: 5,
  },
  {
    mint: 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN',
    symbol: 'JUP',
    name: 'Jupiter',
    refPrice: 0,
    decimals: 6,
  },
  {
    mint: 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm',
    symbol: 'WIF',
    name: 'dogwifhat',
    refPrice: 0,
    decimals: 6,
  },
];

const SYMBOL_BY_MINT = new Map(WATCHLIST.map((t) => [t.mint, t.symbol]));

export function tokenSymbol(mint: string): string {
  return SYMBOL_BY_MINT.get(mint) ?? `${mint.slice(0, 4)}…`;
}

export function tokenMeta(mint: string): WatchToken | undefined {
  return WATCHLIST.find((t) => t.mint === mint);
}
