/** Structured arbitrage opportunity emitted by the OpportunityDetector. */
export interface ArbOpportunity {
    /** Input token mint address. */
    tokenIn: string;
    /** Output token mint (round-trip returns to tokenIn). */
    tokenOut: string;
    /** DEX where we buy (cheaper price). */
    dex1: string;
    /** DEX where we sell (higher price). */
    dex2: string;
    /** Net profit in USD after all fees. */
    profitUsd: number;
    /** Net profit in lamports (for MIN_PROFIT_LAMPORTS filtering). */
    profitLamports: number;
    /** Estimated profit divided by rough risk score (higher = better). */
    profitToRiskRatio: number;
    /** Gross spread between DEX prices in basis points. */
    grossSpreadBps: number;
    /** Breakdown of costs subtracted from gross profit. */
    feesUsd: {
        swapFeesUsd: number;
        txFeeLamports: number;
        priorityFeeLamports: number;
        totalUsd: number;
    };
    /** Execution route: mints in order of the trade. */
    route: string[];
    /** Timestamp when the opportunity was detected. */
    detectedAtMs: number;
    /** Source DEX price (token B per token A). */
    dex1Price: number;
    /** Destination DEX price. */
    dex2Price: number;
}
