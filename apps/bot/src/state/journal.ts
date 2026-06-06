export type JournalEventType =
  | 'scan'
  | 'decision'
  | 'rejection'
  | 'execution'
  | 'reconciliation'
  | 'halt';

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

export class TradeJournal {
  private events: JournalEvent[] = [];
  private readonly maxEvents: number;

  constructor(maxEvents = 5000) {
    this.maxEvents = maxEvents;
  }

  append(event: Omit<JournalEvent, 'timestampMs'> & { timestampMs?: number }): void {
    this.events.push({
      ...event,
      timestampMs: event.timestampMs ?? Date.now(),
    });
    if (this.events.length > this.maxEvents) {
      this.events.splice(0, this.events.length - this.maxEvents);
    }
  }

  all(): readonly JournalEvent[] {
    return this.events;
  }

  byType(type: JournalEventType): JournalEvent[] {
    return this.events.filter((e) => e.type === type);
  }

  exportJson(): string {
    return JSON.stringify(this.events, null, 2);
  }
}
