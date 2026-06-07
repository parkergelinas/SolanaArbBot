import type { JupiterClient } from '../jupiter/client.js';
import type { QuotePairSnapshot } from '../jupiter/types.js';
export interface ScanPair {
    label: string;
    baseMint: string;
    quoteMint: string;
    baseDecimals: number;
    quoteDecimals: number;
}
export declare const DEFAULT_SCAN_PAIRS: ScanPair[];
export interface ArbScanOptions {
    slippageBps?: number;
    restrictIntermediateTokens?: boolean;
}
/**
 * Capture forward + reverse Jupiter Swap v1 quotes for round-trip arb analysis.
 * Each quote call is wrapped in an 8 s timeout so a stalled RPC cannot freeze
 * the engine scan loop.
 */
export declare function scanRoundTripQuotes(client: JupiterClient, pair: ScanPair, tradeAmountUi: number, opts?: ArbScanOptions): Promise<QuotePairSnapshot>;
