/** Runtime configuration — defaults use current Jupiter `api.jup.ag` endpoints. */
const LEGACY_LITE_HOST = 'lite-api.jup.ag';
function env(key, fallback) {
    const v = process.env[key]?.trim();
    return v && v.length > 0 ? v : fallback;
}
function warnIfLegacy(url, label) {
    if (url.includes(LEGACY_LITE_HOST) || url.includes('quote-api.jup.ag')) {
        console.warn(`[config] ${label} uses deprecated Jupiter host — migrate to api.jup.ag (see README)`);
    }
}
export function loadEnv() {
    const jupiterSwapBase = env('JUPITER_SWAP_BASE', 'https://api.jup.ag/swap/v1');
    const jupiterPriceUrl = env('JUPITER_PRICE_URL', 'https://api.jup.ag/price/v3');
    const jupiterTokensBase = env('JUPITER_TOKENS_BASE', 'https://api.jup.ag/tokens/v2');
    warnIfLegacy(jupiterSwapBase, 'JUPITER_SWAP_BASE');
    warnIfLegacy(jupiterPriceUrl, 'JUPITER_PRICE_URL');
    const apiKey = process.env.JUPITER_API_KEY?.trim();
    return {
        jupiterSwapBase: jupiterSwapBase.replace(/\/$/, ''),
        jupiterPriceUrl: jupiterPriceUrl.replace(/\/$/, ''),
        jupiterTokensBase: jupiterTokensBase.replace(/\/$/, ''),
        jupiterApiKey: apiKey && apiKey.length > 0 ? apiKey : undefined,
        rpcUrl: env('SOLANA_RPC_URL', 'https://api.mainnet-beta.solana.com'),
        slippageBps: Number(env('BOT_SLIPPAGE_BPS', '50')),
        minProfitUsd: Number(env('BOT_MIN_PROFIT_USD', '0.25')),
        tradeAmountUi: Number(env('BOT_TRADE_AMOUNT_UI', '1')),
        scanIntervalMs: Number(env('BOT_SCAN_INTERVAL_MS', '2000')),
        paperMode: env('BOT_PAPER_MODE', '1') !== '0',
        walletPublicKey: process.env.BOT_WALLET_PUBKEY?.trim(),
    };
}
/** Well-known mints used by default scanners. */
export const SOL_MINT = 'So11111111111111111111111111111111111111112';
export const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
