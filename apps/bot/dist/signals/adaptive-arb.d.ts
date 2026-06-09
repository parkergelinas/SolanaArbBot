/**
 * Adaptive Arb Strategy — capital-aware multi-source arbitrage that automatically
 * adjusts trade size, spread threshold, and profit gate as capital compounds.
 *
 * Edge sources (in priority order):
 *
 * 1. Pump.fun graduated tokens (first 24–48h on Raydium)
 *    Why: Price discovery window. Raydium listing price ≠ Jupiter best-route price.
 *    Spread: 50–500 bps during active price discovery.
 *    Risk: Token can dump; cap position at 10% of capital per trade.
 *
 * 2. LST de-peg (mSOL/SOL, jitoSOL/SOL)
 *    Why: mSOL has a known fair value = SOL × Marinade exchange rate.
 *    When market price < fair value by > fees: buy mSOL, route back via Jupiter.
 *    Spread: 2–15 bps (small but real and very reliable).
 *    Risk: Minimal — redeemable at fair value within epoch.
 *
 * 3. Cross-DEX spread on active pairs (Raydium vs Orca)
 *    Why: High-volume events temporarily diverge venue prices.
 *    Spread: 0.25–50 bps. Only profitable at Stage 3+ capital.
 *    Risk: Low — symmetric spread, execute both legs atomically via Jito.
 *
 * Adaptive logic:
 *   - trade size, spread threshold, and min-profit gate all scale with capital
 *   - CapitalTracker persists across restarts → compounding survives reboots
 *   - At each capital stage, the binding constraint shifts (fixed tx cost → spread)
 */
import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { ScannableStrategy } from './types.js';
import type { StrategyContext, TradeDecision } from '../strategy/types.js';
import type { PumpTokenRegistry } from '../market/pump-token-registry.js';
import type { CapitalTracker } from '../capital/tracker.js';
export interface AdaptiveArbConfig {
    /** Max capital fraction to risk per trade (default 0.9). */
    maxCapitalFraction: number;
    /** Hard cap on single trade size in SOL regardless of capital. */
    maxTradeSolCap: number;
    /** Number of pairs per scan tick. */
    pairsPerScan: number;
    /** Scanner concurrency. */
    scanConcurrency: number;
}
export declare const DEFAULT_ADAPTIVE_CONFIG: AdaptiveArbConfig;
export declare class AdaptiveArbStrategy implements ScannableStrategy {
    private readonly client;
    private readonly capitalTracker;
    private readonly cfg;
    private readonly pumpRegistry;
    readonly id = "adaptive_arb";
    private scanTick;
    private pairIndex;
    private lastOpportunities;
    constructor(client: JupiterClient, capitalTracker: CapitalTracker, cfg?: AdaptiveArbConfig, pumpRegistry?: PumpTokenRegistry | null);
    scan(state: MarketState): Promise<MarketState>;
    evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null;
    /** Called by engine after a successful execution to compound capital. */
    recordExecuted(profitUsd: number, solPriceUsd: number): void;
    private evaluateOne;
    private nextCoreBatch;
}
