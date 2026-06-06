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
}
export declare function loadEnv(): BotEnv;
/** Well-known mints used by default scanners. */
export declare const SOL_MINT = "So11111111111111111111111111111111111111112";
export declare const USDC_MINT = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
