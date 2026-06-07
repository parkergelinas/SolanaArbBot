/**
 * Pump.fun & memecoin token registry — discovers high-spread Solana tokens
 * using DexScreener's free API.
 *
 * Target tokens:
 *   - Graduated pump.fun tokens (now on Raydium, still volatile)
 *   - Any Solana token with wide Raydium↔Orca price spread
 *   - Low-to-mid cap ($10k–$10M) — large caps are efficiently arb'd already
 *
 * Refresh strategy:
 *   - Full refresh every REFRESH_INTERVAL_MS (default 5 minutes)
 *   - Token list is cached between refreshes
 *   - Pairs are fed into CrossDexArbStrategy for spread measurement
 */
import type { CrossDexScanPair } from '../strategy/cross-dex-scanner.js';
export declare class PumpTokenRegistry {
    private cachedPairs;
    private lastRefresh;
    private refreshTimer;
    /** Start background refresh timer. */
    start(): void;
    stop(): void;
    /** Get current cached pairs, sorted by arb potential. */
    get pairs(): CrossDexScanPair[];
    /** How old the cache is in seconds. */
    get cacheAgeSeconds(): number;
    /** Force a refresh. Called automatically by start(). */
    refresh(): Promise<void>;
}
