export type JournalEventType = 'scan' | 'decision' | 'rejection' | 'execution' | 'reconciliation' | 'halt';
export interface JournalEvent {
    type: JournalEventType;
    timestampMs: number;
    strategyId?: string;
    pairLabel?: string;
    expectedProfitUsd?: number;
    realizedProfitUsd?: number;
    rejectionReason?: string;
    routeMetadata?: Record<string, unknown>;
}
export declare class TradeJournal {
    private events;
    private readonly maxEvents;
    constructor(maxEvents?: number);
    append(event: Omit<JournalEvent, 'timestampMs'> & {
        timestampMs?: number;
    }): void;
    all(): readonly JournalEvent[];
    byType(type: JournalEventType): JournalEvent[];
    exportJson(): string;
}
