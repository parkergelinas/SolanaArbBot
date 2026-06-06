export interface SizingInput {
    baseAmountUi: number;
    spreadBps: number;
    liquidityUsd: number;
    volatilityPct: number;
    routeQualityScore: number;
    recentFailureRate: number;
    minAmountUi: number;
    maxAmountUi: number;
}
/** Adaptive trade size from spread, liquidity, volatility, route depth, failure rate. */
export declare function computeDynamicSize(input: SizingInput): number;
