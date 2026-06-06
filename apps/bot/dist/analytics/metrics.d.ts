import type { JournalEvent } from '../state/journal.js';
export interface AnalyticsSnapshot {
    opportunityHitRate: number;
    avgExpectedProfitUsd: number;
    avgRealizedProfitUsd: number;
    avgQuoteExecutionDriftUsd: number;
    routeWinRates: Record<string, number>;
    rejectionReasons: Record<string, number>;
    totalScans: number;
    totalExecutions: number;
}
export declare function computeAnalytics(events: readonly JournalEvent[]): AnalyticsSnapshot;
