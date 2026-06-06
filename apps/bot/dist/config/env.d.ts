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
}
export declare function loadEnv(): BotEnv;
/** Well-known mints used by default scanners. */
export declare const SOL_MINT = "So11111111111111111111111111111111111111112";
export declare const USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
