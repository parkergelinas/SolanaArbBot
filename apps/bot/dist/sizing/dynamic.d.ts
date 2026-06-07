/**
 * MEV-aware dynamic trade sizer.
 *
 * Core insight: the binding constraint on trade size is not fee dilution (fixed
 * costs per tx are ~$0.002 — negligible) but the MEV sandwich threshold.
 *
 * A sandwich bot profits when:
 *   extractable_value = trade_size × spread_bps/10000 × sol_price_usd > MEV_THRESHOLD
 *
 * Solving for trade_size gives the MEV-safe ceiling for any (spread, sol_price) pair:
 *   mev_safe_sol = MEV_THRESHOLD / (spread_bps / 10_000 × sol_price_usd)
 *
 * With Jito bundle protection active, sandwich attacks are neutralised so we
 * substitute a higher fixed ceiling (JITO_CEILING_SOL) instead.
 *
 * Secondary factors (liquidity depth, failure rate, congestion, route quality)
 * are then applied multiplicatively *downward* from the ceiling — they can only
 * shrink the trade, never push it above the MEV-safe limit.
 */
export interface SizingInput {
    /** Configured base trade size from TRADE_AMOUNT_SOL env var (SOL). */
    baseAmountUi: number;
    /** Gross spread detected by the strategy (bps). */
    spreadBps: number;
    /** Pool/pair liquidity in USD — used to cap size on thin markets. */
    liquidityUsd: number;
    /** Implied volatility proxy (%). Pass 2 if unknown. */
    volatilityPct: number;
    /** Route quality score 0–1 from Jupiter route scoring. */
    routeQualityScore: number;
    /** Fraction of recent trades that failed (0–1). Updated by engine after each trade. */
    recentFailureRate: number;
    /** Absolute minimum trade size (SOL). Trades below this are skipped. */
    minAmountUi: number;
    /** Hard cap regardless of any factor (SOL). Safety net. */
    maxAmountUi: number;
    /** Live SOL price in USD. Used in MEV threshold equation. */
    solPriceUsd: number;
    /** True when a Jito block-engine URL is configured and last submission succeeded. */
    jitoActive: boolean;
    /**
     * When true, skip the trade entirely if Jito is not active rather than falling
     * back to direct RPC. Set this whenever allow_direct_rpc_fallback=false in config.
     */
    requireJito?: boolean;
    /**
     * Recent network priority fee in micro-lamports per CU.
     * High values indicate congestion and elevated MEV activity.
     */
    priorityFeeMicroLamports: number;
}
export interface SizingResult {
    /** Final trade size in SOL (rounded to 2 dp). */
    amountUi: number;
    /** Which constraint was the binding ceiling. */
    ceiling: 'jito' | 'mev_threshold' | 'config_max' | 'min_clamp';
    /** MEV-safe size as computed from the threshold equation (SOL). */
    mevSafeSol: number;
    /** Human-readable explanation of every factor applied. */
    rationale: string;
}
/**
 * Compute the optimal trade size for a single opportunity.
 *
 * Returns a {@link SizingResult} — use `result.amountUi` for the atomic
 * lamport conversion and `result.rationale` for structured logging.
 */
export declare function computeDynamicSize(input: SizingInput): SizingResult;
/** @deprecated Use computeDynamicSize and read result.amountUi */
export declare function computeDynamicSizeUi(input: SizingInput): number;
