export interface DexToken {
  address: string;
  name?: string;
  symbol?: string;
}

export interface DexPair {
  chainId: string;
  dexId: string;
  url: string;
  pairAddress: string;
  baseToken: DexToken;
  quoteToken: DexToken;
  priceUsd?: string;
  priceNative?: string;
  volume?: { h24?: number; h6?: number; h1?: number; m5?: number };
  priceChange?: { h24?: number; h6?: number; h1?: number; m5?: number };
  liquidity?: { usd?: number };
  fdv?: number;
  marketCap?: number;
}

export interface DexPairSnapshot {
  pairAddress: string;
  dexId: string;
  pairUrl: string;
  baseSymbol: string;
  quoteSymbol: string;
  priceUsd: number;
  changeH24Pct: number;
  volumeH24Usd: number;
  liquidityUsd: number;
}

export interface DexTokensResponse {
  pairs?: DexPair[] | null;
}
