import type { JupiterClient } from '../jupiter/client.js';
import type { RouteDivergenceSnapshot } from '../market/state.js';
import { type ArbScanOptions, type ScanPair } from './arb-scanner.js';
/**
 * Capture round-trip quotes under multiple route construction policies and
 * measure divergence between the best and worst outcomes.
 */
export declare function scanRouteDivergence(client: JupiterClient, pair: ScanPair, tradeAmountUi: number, opts?: ArbScanOptions): Promise<RouteDivergenceSnapshot>;
