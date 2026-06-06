export type PriceSource = 'dex' | 'stream' | 'sim' | 'none';

export interface ResolvedPrice {
  priceUsd: number;
  changeH24Pct: number;
  volumeH24Usd: number;
  source: PriceSource;
  stale: boolean;
}

export interface DexAnchor {
  priceUsd: number;
  changeH24Pct: number;
  volumeH24Usd: number;
  liquidityUsd: number;
  fetchedAt: number;
}
