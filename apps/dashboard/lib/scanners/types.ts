/** Wallet flagged for oversized shitcoin purchases (Solscan researcher). */
export interface ShitcoinWhaleHit {
  id: string;
  wallet: string;
  tokenMint: string;
  tokenSymbol: string;
  amountSol: number;
  amountUsd: number;
  marketCapUsd: number;
  liquidityUsd: number;
  detectedAtMs: number;
  signature?: string;
  source: 'solscan' | 'helius' | 'dexscreener';
  score: number;
  solscanUrl: string;
  tokenUrl: string;
}

/** Early pump.fun bonding-curve momentum before UI prominence. */
export interface PumpMomentumHit {
  id: string;
  mint: string;
  symbol: string;
  name: string;
  pairAddress: string;
  ageMinutes: number;
  volumeM5Usd: number;
  volumeH1Usd: number;
  buysM5: number;
  buysH1: number;
  liquidityUsd: number;
  marketCapUsd: number;
  graduationPct: number;
  priceChangeM5Pct: number;
  priceChangeH1Pct: number;
  momentumScore: number;
  detectedAtMs: number;
  dexUrl: string;
  pumpUrl: string;
  /** Seen via Helius logsSubscribe / RPC before DexScreener prominence. */
  onChain?: boolean;
  signature?: string;
}

export interface ScannerMeta {
  source: string;
  degraded: boolean;
  message?: string;
  apiKeys: {
    solscan: boolean;
    helius: boolean;
  };
  polledAtMs: number;
}

export interface SolscanResearcherResponse {
  hits: ShitcoinWhaleHit[];
  meta: ScannerMeta;
}

export interface PumpScannerResponse {
  hits: PumpMomentumHit[];
  meta: ScannerMeta;
}
