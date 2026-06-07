/**
 * Cross-DEX price scanner — compares Raydium vs Orca quotes for the same pair
 * using Jupiter's `dexes` parameter to force DEX-specific routing.
 *
 * Edge logic:
 *   1. Get quote for A→B restricted to Raydium pools only
 *   2. Get quote for A→B restricted to Orca/Whirlpool pools only
 *   3. Spread = abs(raydiumPrice - orcaPrice) / min(prices) * 10_000 (bps)
 *   4. Profitable if spread > combined fee floor (~30 bps round-trip)
 *
 * Execution model:
 *   Paper mode  — simulate both legs, record spread
 *   Live mode   — requires Jito bundle (two atomic txs) to avoid execution risk
 *                 between the buy-leg and sell-leg
 */
import type { JupiterClient } from '../jupiter/client.js';
import type { SwapQuoteResponse } from '../jupiter/types.js';
export declare const RAYDIUM_DEXES = "Raydium,Raydium CLMM,Raydium CP";
export declare const ORCA_DEXES = "Orca,Whirlpool";
export declare const CROSS_DEX_FEE_FLOOR_BPS = 30;
export interface CrossDexQuote {
    pairLabel: string;
    baseMint: string;
    quoteMint: string;
    baseDecimals: number;
    quoteDecimals: number;
    amountInUi: number;
    raydiumQuote: SwapQuoteResponse | null;
    orcaQuote: SwapQuoteResponse | null;
    /** Raydium output per unit input (quote tokens per base token). */
    raydiumPriceOut: number;
    /** Orca output per unit input. */
    orcaPriceOut: number;
    /** Absolute spread in bps between the two venues. */
    spreadBps: number;
    /** Net spread after estimated combined fees. */
    netSpreadBps: number;
    /**
     * Which DEX gives MORE output tokens (buy here, you get more).
     * To arb: buy on cheaperDex (sell base for more quote), sell on dearerDex.
     */
    cheaperDex: 'raydium' | 'orca' | 'equal';
    dearerDex: 'raydium' | 'orca' | 'equal';
    /** True when netSpreadBps > 0 and both venues have liquidity. */
    profitable: boolean;
    capturedAtMs: number;
}
export interface CrossDexScanResult {
    opportunities: CrossDexQuote[];
    scannedAt: number;
    errors: Array<{
        pairLabel: string;
        error: string;
    }>;
}
export interface CrossDexScanPair {
    label: string;
    baseMint: string;
    quoteMint: string;
    baseDecimals: number;
    quoteDecimals: number;
}
/**
 * Scan a single pair for cross-DEX price divergence between Raydium and Orca.
 */
export declare function scanCrossDexPair(client: JupiterClient, pair: CrossDexScanPair, amountUi: number, slippageBps?: number): Promise<CrossDexQuote>;
/**
 * Scan multiple pairs concurrently, respecting a concurrency limit.
 */
export declare function scanCrossDexPairs(client: JupiterClient, pairs: CrossDexScanPair[], amountUi: number, opts?: {
    slippageBps?: number;
    concurrency?: number;
}): Promise<CrossDexScanResult>;
