import type { JupiterClient } from '../jupiter/client.js';
import type { QuotePairSnapshot } from '../jupiter/types.js';
import { type ArbScanOptions, type ScanPair } from './arb-scanner.js';
import type { RouteDivergenceSnapshot } from '../market/state.js';
export interface MultiPairScanResult {
    quotes: Map<string, QuotePairSnapshot>;
    divergences: Map<string, RouteDivergenceSnapshot>;
    errors: Map<string, string>;
}
export interface MultiPairScanOptions extends ArbScanOptions {
    /** Max concurrent Jupiter quote requests. */
    concurrency?: number;
    /** Delay between batches (ms) to respect rate limits. */
    batchDelayMs?: number;
}
/** Scan round-trip quotes across multiple pairs with concurrency control. */
export declare function scanMultipleRoundTrips(client: JupiterClient, pairs: ScanPair[], tradeAmountUi: number, opts?: MultiPairScanOptions): Promise<MultiPairScanResult>;
/** Scan route divergence across multiple pairs. */
export declare function scanMultipleRouteDivergences(client: JupiterClient, pairs: ScanPair[], tradeAmountUi: number, opts?: MultiPairScanOptions): Promise<MultiPairScanResult>;
