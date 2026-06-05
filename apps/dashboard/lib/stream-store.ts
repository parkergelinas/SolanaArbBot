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
  WsBatchFrame,
  WsEvent,
} from './types';
import { isWsBatchFrame, isWsEvent } from './types';

const FLUSH_MS = 33;
const MAX_SIGNALS = 1_000;

type Listener = () => void;

class StreamStore {
  private version = 0;
  private listeners = new Set<Listener>();
  private inbox: WsEvent[] = [];
  private flushTimer: ReturnType<typeof setTimeout> | null = null;

  private signals: SignalEvent[] = [];
  private latest = new Map<WsEvent['type'], unknown>();

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
      } else {
        this.latest.set(ev.type, ev.data);
      }
    }

    if (this.signals.length > MAX_SIGNALS) {
      this.signals = this.signals.slice(-MAX_SIGNALS);
    }

    this.inbox = [];
    this.version += 1;
    this.listeners.forEach((l) => l());
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  getVersion(): number {
    return this.version;
  }

  getSignalsSnapshot(maxItems: number): SignalEvent[] {
    void this.version;
    return this.signals.slice(-maxItems);
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
