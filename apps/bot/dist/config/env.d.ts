/** Runtime configuration — defaults use current Jupiter `api.jup.ag` endpoints. */
export interface BotEnv {
    jupiterSwapBase: string;
    jupiterPriceUrl: string;
    jupiterTokensBase: string;
    jupiterApiKey?: string;
    rpcUrl: string;
    slippageBps: number;
    minProfitUsd: number;
    tradeAmountUi: number;
    scanIntervalMs: number;
    paperMode: boolean;
    /**
     * Live-quotes mode — use real Jupiter quote API for divergence measurement
     * while keeping execution simulated (paperMode=true required).
     * Validates real route divergence before committing capital on mainnet.
     */
    liveQuotes: boolean;
    walletPublicKey?: string;
    /** Primary strategy id — `route_divergence_arb` | `round_trip_quote_arb` */
    primaryStrategy: string;
    enableMeanReversion: boolean;
    /** Stub only — Jupiter Trigger API (TP/SL/breakout). */
    enableTriggerApi: boolean;
    /** Stub only — Jupiter Recurring API (DCA/treasury). */
    enableRecurringApi: boolean;
    /** CoinMarketCap Pro API key for top-100 pair loading. */
    cmcApiKey?: string;
    /** Pair source: `cmc` (live API) or `static` (built-in map). */
    pairSource: 'cmc' | 'static';
    /** Max scan pairs including SOL/USDC. */
    maxScanPairs: number;
    /** Pairs scanned per engine tick (rate-limit rotation). */
    pairsPerScan: number;
    /** Quote mint for alt pairs: USDC or SOL. */
    pairQuoteMint: 'USDC' | 'SOL';
    /** CMC listings to fetch when pairSource=cmc. */
    cmcTopN: number;
    /** Enable Pump.fun bonding curve edge strategy. */
    enablePumpEdge: boolean;
    /** Helius API key for pump launch monitoring. */
    heliusApiKey?: string;
    /** Enable cross-DEX arb strategy (Raydium vs Orca price comparison). */
    enableCrossDexArb: boolean;
    /** Min spread bps between Raydium and Orca to signal a core-pair trade. */
    crossDexSpreadBps: number;
    /** Min spread bps for pump.fun/memecoin pairs (wider threshold = higher edge). */
    pumpSpreadBps: number;
    /** Enable pump.fun memecoin spread scanning via DexScreener. */
    enablePumpSpreads: boolean;
    /** Enable Jito bundle submission for MEV protection. */
    jitoEnabled: boolean;
    /** Jito tip in lamports added to arb bundles. */
    jitoTipLamports: number;
    /** Minimum net profit in lamports required to execute a trade. */
    minProfitLamports: number;
    /** Port for the Express monitoring dashboard. */
    monitorPort: number;
    /** Pino log level (trace | debug | info | warn | error). */
    logLevel: string;
    /** Path to the SQLite trade log database. */
    sqlitePath: string;
    /** PnL drop in SOL that triggers a Telegram/Discord alert (negative, e.g. -0.1). */
    alertPnlThresholdSol: number;
    /** Comma-separated Orca Whirlpool pool addresses to monitor. */
    orcaPoolAddresses: string[];
    /** Spread in bps that triggers an arb opportunity event. Default 80 (0.8%). */
    spreadThresholdBps: number;
}
export declare function loadEnv(): BotEnv;
/** Well-known mints used by default scanners. */
export declare const SOL_MINT = "So11111111111111111111111111111111111111112";
export declare const USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
