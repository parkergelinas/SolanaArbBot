/**
 * External stream store — batches inbound WebSocket events before notifying React.
 * One flush per ~33 ms tick; components subscribe via useSyncExternalStore.
 */

import type {
  HealthStatus,
  Portfolio,
  Risk,
  SignalEvent,
  SystemStatus,
  TradeEvent,
  WsBatchFrame,
  WsEvent,
} from './types';
import { isWsBatchFrame, isWsEvent } from './types';

const FLUSH_MS = 33;
const MAX_SIGNALS = 1_000;
const MAX_TRADE_EVENTS = 2_000;

type Listener = () => void;

class StreamStore {
  private version = 0;
  private listeners = new Set<Listener>();
  private inbox: WsEvent[] = [];
  private flushTimer: ReturnType<typeof setTimeout> | null = null;

  private signals: SignalEvent[] = [];
  private tradeEvents: TradeEvent[] = [];
  /** Latest lifecycle stage per trade_id. */
  private tradeById = new Map<string, TradeEvent>();
  private latestTrade: TradeEvent | null = null;
  private latest = new Map<WsEvent['type'], unknown>();
  /** Cached snapshots — useSyncExternalStore requires stable references between flushes. */
  private snapshotCache = new Map<string, SignalEvent[]>();
  private tradesSnapshotCache = new Map<string, TradeEvent[]>();
  private snapshotCacheVersion = -1;

  /** Parse a raw WebSocket text frame (batch or legacy single event). */
  ingestRaw(text: string) {
    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch {
      return;
    }

    if (isWsBatchFrame(parsed)) {
      this.enqueue(parsed.events);
      return;
    }
    if (isWsEvent(parsed)) {
      this.enqueue([parsed]);
    }
  }

  private enqueue(events: WsEvent[]) {
    if (events.length === 0) return;
    this.inbox.push(...events);
    this.scheduleFlush();
  }

  private scheduleFlush() {
    if (this.flushTimer !== null) return;
    this.flushTimer = setTimeout(() => this.flush(), FLUSH_MS);
  }

  private flush() {
    this.flushTimer = null;
    if (this.inbox.length === 0) return;

    for (const ev of this.inbox) {
      if (ev.type === 'signal') {
        this.signals.push(ev.data);
      } else if (ev.type === 'trade') {
        this.applyTrade(ev.data);
      } else {
        this.latest.set(ev.type, ev.data);
      }
    }

    if (this.signals.length > MAX_SIGNALS) {
      this.signals = this.signals.slice(-MAX_SIGNALS);
    }
    if (this.tradeEvents.length > MAX_TRADE_EVENTS) {
      this.tradeEvents = this.tradeEvents.slice(-MAX_TRADE_EVENTS);
    }

    this.inbox = [];
    this.version += 1;
    this.listeners.forEach((l) => l());
  }

  private applyTrade(trade: TradeEvent) {
    this.tradeEvents.push(trade);
    this.tradeById.set(trade.trade_id, trade);
    this.latestTrade = trade;
    this.latest.set('trade', trade);
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  getVersion(): number {
    return this.version;
  }

  private invalidateSnapshots() {
    if (this.snapshotCacheVersion !== this.version) {
      this.snapshotCache.clear();
      this.tradesSnapshotCache.clear();
      this.snapshotCacheVersion = this.version;
    }
  }

  getTradesSnapshot(maxItems: number): TradeEvent[] {
    this.invalidateSnapshots();
    const key = String(maxItems);
    let snapshot = this.tradesSnapshotCache.get(key);
    if (!snapshot) {
      snapshot = Array.from(this.tradeById.values())
        .sort((a, b) => b.timestamp_us - a.timestamp_us)
        .slice(0, maxItems);
      this.tradesSnapshotCache.set(key, snapshot);
    }
    return snapshot;
  }

  getLatestTrade(): TradeEvent | null {
    void this.version;
    return this.latestTrade;
  }

  getSignalsSnapshot(maxItems: number): SignalEvent[] {
    this.invalidateSnapshots();
    const key = String(maxItems);
    let snapshot = this.snapshotCache.get(key);
    if (!snapshot) {
      snapshot = this.signals.slice(-maxItems);
      this.snapshotCache.set(key, snapshot);
    }
    return snapshot;
  }

  getLatest<T>(type: WsEvent['type']): T | null {
    void this.version;
    return (this.latest.get(type) as T) ?? null;
  }

  /** Typed shortcuts for common panels */
  get health(): HealthStatus | null {
    return this.getLatest<HealthStatus>('health');
  }

  get status(): SystemStatus | null {
    return this.getLatest<SystemStatus>('status');
  }

  get portfolio(): Portfolio | null {
    return this.getLatest<Portfolio>('portfolio');
  }

  get risk(): Risk | null {
    return this.getLatest<Risk>('risk');
  }
}

export const streamStore = new StreamStore();

export type { WsBatchFrame };
