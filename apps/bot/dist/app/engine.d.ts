import type { Strategy } from '../signals/types.js';
import { TradeJournal } from '../state/journal.js';
export interface EngineStats {
    scans: number;
    actionable: number;
    executed: number;
    rejected: number;
    pairCount: number;
}
/**
 * Main orchestrator — 4-layer pipeline:
 * MarketData → Signal → Risk → Execution
 */
export declare class BotEngine {
    private readonly env;
    private readonly journal;
    private readonly registry;
    private readonly stack;
    private readonly pairRegistry;
    private readonly executor;
    private readonly hardenedExecutor;
    private readonly opportunityDetector;
    private stopMonitor;
    private inFlightCount;
    private readonly scannable;
    private pumpStrategy;
    private riskState;
    /** Dedicated Connection for priority-fee sampling (separate from JupiterClient internals). */
    private readonly connection;
    private stats;
    private failureRate;
    private running;
    private tradeAmountUi;
    private initialized;
    /** Cached SOL price — refreshed every scan, used by event-driven arb handler. */
    private lastSolPriceUsd;
    /** Cached priority fee — refreshed every N scans to avoid per-scan RPC overhead. */
    private lastPriorityFeeMicroLamports;
    private priorityFeeRefreshAt;
    constructor();
    getStats(): EngineStats;
    getJournal(): TradeJournal;
    getAnalytics(): import("../analytics/metrics.js").AnalyticsSnapshot;
    getStrategies(): readonly Strategy[];
    getPairCount(): number;
    private ensureInitialized;
    scanOnce(): Promise<void>;
    private pickDecision;
    /**
     * Execute a cross-DEX opportunity emitted by the OpportunityDetector.
     * Runs outside the scan loop (event-driven), so uses its own inFlightCount slot.
     */
    private handleExternalOpportunity;
    runLoop(maxIterations?: number): Promise<void>;
    /**
     * Graceful shutdown — waits for in-flight trades to complete before stopping.
     * Called by SIGTERM/SIGINT handlers in index.ts.
     */
    gracefulStop(timeoutMs?: number): Promise<void>;
    stop(): void;
}
