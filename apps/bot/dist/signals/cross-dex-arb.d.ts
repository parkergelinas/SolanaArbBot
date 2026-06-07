/**
 * Cross-DEX arb strategy — compares Raydium vs Orca real-time prices for the
 * same token pair and signals when the spread exceeds combined fee cost.
 *
 * Two pair sources:
 *   1. Static core pairs (SOL/USDC, JUP/USDC, mSOL/SOL, etc.) — always scanned
 *   2. Pump.fun memecoin registry (PumpTokenRegistry) — rotated in when enabled;
 *      these have wider spreads (50–500 bps) but higher risk
 *
 * Execution model:
 *   Paper  — simulate both legs, record spread in journal for analysis
 *   Live   — Jito bundle: tx1 = buy on cheaperDex, tx2 = sell on dearerDex
 *            (two-transaction atomic bundle; execution gap risk eliminated)
 *
 * Spread thresholds:
 *   Core pairs:   BOT_CROSS_DEX_SPREAD_BPS (default 35 bps — above fee floor)
 *   Pump tokens:  BOT_PUMP_SPREAD_BPS (default 80 bps — wider, riskier)
 */
import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { ScannableStrategy } from './types.js';
import type { StrategyContext, TradeDecision } from '../strategy/types.js';
import { type CrossDexScanPair } from '../strategy/cross-dex-scanner.js';
import type { PumpTokenRegistry } from '../market/pump-token-registry.js';
export declare const CROSS_DEX_CORE_PAIRS: CrossDexScanPair[];
export interface CrossDexArbConfig {
    /** Min spread bps for core pairs to signal a trade. */
    coreSpreadThresholdBps: number;
    /** Min spread bps for pump/memecoin pairs to signal a trade. */
    pumpSpreadThresholdBps: number;
    /** Max pairs to scan per tick (controls API rate usage). */
    pairsPerScan: number;
    /** Concurrency for parallel quote fetching. */
    scanConcurrency: number;
    /** Whether to scan pump/memecoin pairs from PumpTokenRegistry. */
    enablePumpPairs: boolean;
}
export declare const DEFAULT_CROSS_DEX_CONFIG: CrossDexArbConfig;
export declare class CrossDexArbStrategy implements ScannableStrategy {
    private readonly client;
    private readonly tradeAmountUi;
    private readonly cfg;
    private readonly pumpRegistry;
    readonly id = "cross_dex_arb";
    private scanTick;
    private corePairIndex;
    private lastOpportunities;
    constructor(client: JupiterClient, tradeAmountUi: number, cfg?: CrossDexArbConfig, pumpRegistry?: PumpTokenRegistry | null);
    scan(state: MarketState): Promise<MarketState>;
    evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null;
    private evaluateOne;
    /** Rotate through core pairs in batches. */
    private nextCoreBatch;
}
