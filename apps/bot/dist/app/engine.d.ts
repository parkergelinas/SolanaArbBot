import type { Strategy } from '../signals/types.js';
import { TradeJournal } from '../state/journal.js';
export interface EngineStats {
    scans: number;
    actionable: number;
    executed: number;
    rejected: number;
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
    private readonly executor;
    private readonly scannable;
    private riskState;
    private stats;
    private failureRate;
    private running;
    private tradeAmountUi;
    constructor();
    getStats(): EngineStats;
    getJournal(): TradeJournal;
    getAnalytics(): import("../analytics/metrics.js").AnalyticsSnapshot;
    getStrategies(): readonly Strategy[];
    scanOnce(): Promise<void>;
    private pickDecision;
    runLoop(maxIterations?: number): Promise<void>;
    stop(): void;
}
