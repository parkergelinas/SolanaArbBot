/**
 * WebSocket stream client — buffers inbound messages, flushes on requestAnimationFrame.
 * Never invokes subscribers synchronously from onmessage.
 */

import { isWSBatchFrame, type WSBatchFrame, type WSMessage } from './types';

export type StreamFlushMeta = {
  seq: number;
  ts_ms: number;
  receivedAt: number;
  flushedAt: number;
  messageCount: number;
};

export type MessageHandler = (messages: WSMessage[], meta: StreamFlushMeta) => void;

export type StreamLifecycle = {
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (err: unknown) => void;
  onFrameReceived?: (meta: { seq: number; ts_ms: number; receivedAt: number }) => void;
};

type Subscriber = MessageHandler;

export class StreamClient {
  private ws: WebSocket | null = null;
  private url = '';
  private inbox: WSMessage[] = [];
  private rafId: number | null = null;
  private subscribers = new Set<Subscriber>();
  private lifecycle: StreamLifecycle = {};
  private lastMeta: Omit<StreamFlushMeta, 'flushedAt' | 'messageCount'> = {
    seq: 0,
    ts_ms: 0,
    receivedAt: 0,
  };
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  connect(url: string, lifecycle: StreamLifecycle = {}) {
    this.url = url;
    this.lifecycle = lifecycle;
    this.openSocket();
  }

  disconnect() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
    this.ws?.close();
    this.ws = null;
    this.inbox = [];
  }

  subscribe(handler: MessageHandler): () => void {
    this.subscribers.add(handler);
    return () => this.subscribers.delete(handler);
  }

  onMessage(raw: string) {
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw);
    } catch {
      return;
    }
    if (!isWSBatchFrame(parsed)) return;
    this.ingestBatch(parsed);
  }

  private ingestBatch(frame: WSBatchFrame) {
    if (frame.messages.length === 0) return;
    const receivedAt = performance.now();
    this.lastMeta = { seq: frame.seq, ts_ms: frame.ts_ms, receivedAt };
    this.lifecycle.onFrameReceived?.({
      seq: frame.seq,
      ts_ms: frame.ts_ms,
      receivedAt,
    });
    this.inbox.push(...frame.messages);
    this.scheduleFlush();
  }

  private scheduleFlush() {
    if (this.rafId !== null) return;
    if (typeof requestAnimationFrame === 'undefined') {
      this.flush();
      return;
    }
    this.rafId = requestAnimationFrame(() => {
      this.rafId = null;
      this.flush();
    });
  }

  private flush() {
    if (this.inbox.length === 0) return;
    const batch = this.inbox;
    this.inbox = [];
    const flushedAt = performance.now();
    const meta: StreamFlushMeta = {
      ...this.lastMeta,
      flushedAt,
      messageCount: batch.length,
    };
    this.subscribers.forEach((sub) => sub(batch, meta));
  }

  private openSocket() {
    if (typeof WebSocket === 'undefined') return;

    const ws = new WebSocket(this.url);
    this.ws = ws;

    ws.onopen = () => this.lifecycle.onOpen?.();
    ws.onmessage = (ev: MessageEvent) => this.onMessage(ev.data as string);
    ws.onerror = () => this.lifecycle.onError?.(new Error('WebSocket error'));
    ws.onclose = () => {
      this.lifecycle.onClose?.();
      this.ws = null;
      this.reconnectTimer = setTimeout(() => this.openSocket(), 3_000);
    };
  }
}

export const streamClient = new StreamClient();
