/**
 * Auto-tuner — analyses spread data accumulated during a paper session and
 * produces concrete parameter recommendations for each capital stage.
 *
 * This is intentionally read-only: it never applies changes automatically.
 * Recommendations are surfaced via the /analytics and /backtest endpoints.
 */
import type { JournalEvent } from '../state/journal.js';
export interface StageRecommendation {
    stage: string;
    currentMinSpreadBps: number;
    recommendedMinSpreadBps: number;
    currentMinProfitUsd: number;
    recommendedMinProfitUsd: number;
    breakEvenBps: number;
    estimatedTradesPerHour: number;
    estimatedHourlyProfitUsd: number;
    viable: boolean;
    viabilityNote: string;
}
export interface TunerReport {
    generatedAt: number;
    solPriceUsd: number;
    hoursOfData: number;
    totalObservations: number;
    stageRecommendations: StageRecommendation[];
    topSuggestion: string;
    needsPumpData: boolean;
}
export declare function runAutoTuner(events: readonly JournalEvent[], solPriceUsd?: number): TunerReport;
