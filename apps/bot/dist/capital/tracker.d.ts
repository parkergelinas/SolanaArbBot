/**
 * Capital tracker — persists simulated/live balance across restarts and
 * emits adaptive strategy parameters that scale automatically as capital grows.
 *
 * Design philosophy:
 *   The binding constraint at small capital is FIXED transaction cost (~$0.002/tx).
 *   At $0.002 fixed cost, you need:
 *     $0.01 profit target → spread ≥ 102 bps on $0.10 notional  (impractical)
 *     $0.01 profit target → spread ≥ 10  bps on $1.00 notional  (marginal)
 *     $0.01 profit target → spread ≥ 2   bps on $5.00 notional  (achievable)
 *   So the strategy gets MORE viable as capital grows — not less.
 *
 * Capital stages (SOL balance):
 *   Stage 1:  1–3  SOL — minimum viable, target $0.01–$0.05/trade
 *   Stage 2:  3–10 SOL — $0.05–$0.20/trade
 *   Stage 3: 10–30 SOL — $0.10–$0.50/trade
 *   Stage 4: 30+   SOL — $0.20–$1.00/trade, full strategy suite
 */
export interface AdaptiveParams {
    /** How much SOL to trade per opportunity. */
    tradeSizeUi: number;
    /** Minimum spread in bps to consider a trade. */
    minSpreadBps: number;
    /** Minimum net profit in USD after fees to execute. */
    minProfitUsd: number;
    /** Human label for current stage. */
    stage: string;
    /** Current tracked capital in SOL. */
    capitalSol: number;
}
export interface CapitalState {
    capitalSol: number;
    totalProfitUsd: number;
    tradeCount: number;
}
export declare class CapitalTracker {
    private db;
    private state;
    private readonly solPriceUsd;
    constructor(dbPath: string, startingCapitalSol: number, solPriceUsd: number);
    get capitalSol(): number;
    /** Record a completed trade and update capital. profitUsd can be negative. */
    recordTrade(profitUsd: number, solPriceUsd: number): void;
    /**
     * Compute adaptive strategy parameters for the current capital level.
     *
     * The key insight: as capital doubles, the same spread generates 2× the
     * dollar profit per trade. This means we can tighten the spread threshold
     * over time and trade more frequently on tighter opportunities.
     */
    getAdaptiveParams(solPriceUsd?: number): AdaptiveParams;
    get stats(): CapitalState;
    close(): void;
}
