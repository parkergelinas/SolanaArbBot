import { TradeJournal } from '../state/journal.js';
export interface EngineStats {
    scans: number;
    actionable: number;
    executed: number;
    rejected: number;
}
/** Main scan loop — market data → strategy → risk → sizing → execution. */
export declare class BotEngine {
    private readonly env;
    private readonly journal;
    private readonly registry;
    private readonly stack;
    private readonly executor;
    private riskState;
    private stats;
    private failureRate;
    private running;
    private readonly quoteArb;
    private tradeAmountUi;
    constructor();
    getStats(): EngineStats;
    getJournal(): TradeJournal;
    getAnalytics(): import("../analytics/metrics.js").AnalyticsSnapshot;
    scanOnce(): Promise<void>;
    runLoop(maxIterations?: number): Promise<void>;
    stop(): void;
}
