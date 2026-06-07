/**
 * Static fallback: CMC top-100 symbols → Solana mint addresses.
 * Used when CMC API key is absent or info endpoint returns no contract.
 * Sources: Jupiter verified list + Solscan token registry.
 */

export interface StaticSolanaToken {
  symbol: string;
  name: string;
  mint: string;
  decimals: number;
  cmcRankHint?: number;
}

/** Well-known Solana-native / wrapped tokens from CMC top 100. */
export const STATIC_SOLANA_TOKENS: StaticSolanaToken[] = [
  { symbol: 'SOL', name: 'Solana', mint: 'So11111111111111111111111111111111111111112', decimals: 9, cmcRankHint: 5 },
  { symbol: 'USDC', name: 'USD Coin', mint: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', decimals: 6, cmcRankHint: 6 },
  { symbol: 'USDT', name: 'Tether', mint: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', decimals: 6, cmcRankHint: 3 },
  { symbol: 'BONK', name: 'Bonk', mint: 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263', decimals: 5, cmcRankHint: 80 },
  { symbol: 'JUP', name: 'Jupiter', mint: 'JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN', decimals: 6, cmcRankHint: 70 },
  { symbol: 'WIF', name: 'dogwifhat', mint: 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm', decimals: 6, cmcRankHint: 50 },
  { symbol: 'RAY', name: 'Raydium', mint: '4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R', decimals: 6 },
  { symbol: 'PYTH', name: 'Pyth Network', mint: 'HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3', decimals: 6 },
  { symbol: 'JTO', name: 'Jito', mint: 'jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL', decimals: 9 },
  { symbol: 'ORCA', name: 'Orca', mint: 'orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE', decimals: 6 },
  { symbol: 'RENDER', name: 'Render', mint: 'rndrizKT3MK1iimdxRdWabcF7Zg7AR5T4nud4EkHBof', decimals: 8 },
  { symbol: 'HNT', name: 'Helium', mint: 'hntyVP6YFm1Hg25TN9WGLqM12AB1aCwaoa9EwX4MH3J', decimals: 8 },
  { symbol: 'POPCAT', name: 'Popcat', mint: '7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr', decimals: 9 },
  { symbol: 'MEW', name: 'cat in a dogs world', mint: 'MEW1gQWJ3nEXg2qgERiKu7FAFj79PHv5BPqqH93c5rY', decimals: 5 },
  { symbol: 'FARTCOIN', name: 'Fartcoin', mint: '9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump', decimals: 6 },
  { symbol: 'TRUMP', name: 'OFFICIAL TRUMP', mint: '6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN', decimals: 6 },
  { symbol: 'PENGU', name: 'Pudgy Penguins', mint: '2zMMhcVQEXDtdE6vsFS7S7D5oUodfK5vUhteWS8pmeRG', decimals: 6 },
  { symbol: 'GRASS', name: 'Grass', mint: 'Grass7B4RdKfBCjTKgSqnXkqjwiGvQyFbuSCUJr2XX58', decimals: 9 },
  { symbol: 'IO', name: 'io.net', mint: 'BZLbGTNCSFfoth2GYDtwr7e4imWzpR5jqcUuGEwr646K', decimals: 8 },
  { symbol: 'DRIFT', name: 'Drift', mint: 'DriFtupJYLTosbwoN8koMbEYSx54aFAVLddW1yMmeKht', decimals: 6 },
  { symbol: 'MPLX', name: 'Metaplex', mint: 'METAewgxyPbgwsseH8T36aNaNvxSijGxNJgM7zNhUrn', decimals: 6 },
  { symbol: 'MNDE', name: 'Marinade', mint: 'MNDEFzGvMt87ueuHvVU9VcTqsAP5b3fTGPsHuuPA5ey', decimals: 9 },
  { symbol: 'MSOL', name: 'Marinade Staked SOL', mint: 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So', decimals: 9 },
  { symbol: 'JITOSOL', name: 'Jito Staked SOL', mint: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn', decimals: 9 },
  { symbol: 'BSOL', name: 'BlazeStake Staked SOL', mint: 'bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1', decimals: 9 },
  { symbol: 'TNSR', name: 'Tensor', mint: 'TNSRxcUxoT9xBG3de7PiJyTDYu7kskLqcpddxnEJAS6', decimals: 9 },
  { symbol: 'KMNO', name: 'Kamino', mint: 'KMNo3nJsBXfcpJTVhZcXLW7RmTwTt4GVFE7suUBo9sS', decimals: 6 },
  { symbol: 'MOBILE', name: 'Helium Mobile', mint: 'mb1eu7TzEc71KxDpsMko1zx7vdDrp6YMUrkoS2gaCbAe', decimals: 6 },
  { symbol: 'IOT', name: 'Helium IOT', mint: 'iotEVSKZptdovn9zZ9fr8WEFGFbS8iqi7RitqqP3W7Sn', decimals: 6 },
  { symbol: 'SHDW', name: 'Shadow Token', mint: 'SHDWyBxihqiKMfPPRKx26Hj4mHYkCsauzT5EGwZ1B8p', decimals: 9 },
  { symbol: 'SAMO', name: 'Samoyedcoin', mint: '7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU', decimals: 9 },
  { symbol: 'GMT', name: 'STEPN', mint: '7i5KKsX2weiTkry7jA4ZwSuJH8PUv8uKDDa1LS54L5Vn', decimals: 9 },
  { symbol: 'STEP', name: 'Step Finance', mint: 'StepAscQoEioFxxWGnh2sLBDFp9d8rvKz2Ypzy1Grop', decimals: 9 },
  { symbol: 'SRM', name: 'Serum', mint: 'SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt', decimals: 6 },
  { symbol: 'COPE', name: 'Cope', mint: '8HGyAAB1yoM1ttS7pXjHMa3dukTFGQggnFFy3tHS7PX', decimals: 6 },
  { symbol: 'MEDIA', name: 'Media Network', mint: 'ETAtLmCmsoiEEKfNrHKw2k4v2X7vKcZ5vUsKXsC1m1iq', decimals: 6 },
];

const BY_SYMBOL = new Map(STATIC_SOLANA_TOKENS.map((t) => [t.symbol.toUpperCase(), t]));
const BY_MINT = new Map(STATIC_SOLANA_TOKENS.map((t) => [t.mint, t]));

export function staticMintForSymbol(symbol: string): StaticSolanaToken | undefined {
  return BY_SYMBOL.get(symbol.toUpperCase());
}

export function staticTokenForMint(mint: string): StaticSolanaToken | undefined {
  return BY_MINT.get(mint);
}
