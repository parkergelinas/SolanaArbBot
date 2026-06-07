import { PUMP_PROGRAM_ID } from './constants.js';

export interface PumpLaunchEvent {
  mint: string;
  signature: string;
  slot: number;
  creator?: string;
  detectedAtMs: number;
}

type LaunchHandler = (event: PumpLaunchEvent) => void;

/**
 * Monitor Pump.fun `Create` instructions via Helius logsSubscribe.
 * Docs: pump-public-docs — `create(user, name, symbol, uri, creator)`.
 */
const RECONNECT_BASE_MS = 1_000;
const RECONNECT_MAX_MS = 60_000;
const RECONNECT_MAX_ATTEMPTS = 20;

export class PumpLaunchMonitor {
  private ws: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private running = false;
  private reconnectAttempts = 0;

  constructor(
    private readonly heliusWsUrl: string,
    private readonly onLaunch: LaunchHandler,
  ) {}

  start(): void {
    if (this.running) return;
    this.running = true;
    this.connect();
  }

  stop(): void {
    this.running = false;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.ws?.close();
    this.ws = null;
  }

  private scheduleReconnect(): void {
    if (!this.running) return;
    if (this.reconnectAttempts >= RECONNECT_MAX_ATTEMPTS) {
      console.warn(
        `[PumpLaunchMonitor] giving up after ${RECONNECT_MAX_ATTEMPTS} reconnect attempts`,
      );
      return;
    }
    const delay = Math.min(
      RECONNECT_BASE_MS * 2 ** this.reconnectAttempts,
      RECONNECT_MAX_MS,
    );
    this.reconnectAttempts += 1;
    console.warn(
      `[PumpLaunchMonitor] reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${RECONNECT_MAX_ATTEMPTS})`,
    );
    this.reconnectTimer = setTimeout(() => this.connect(), delay);
  }

  private connect(): void {
    if (!this.running) return;

    const ws = new WebSocket(this.heliusWsUrl);
    this.ws = ws;

    ws.onopen = () => {
      // Reset backoff counter on every successful connection
      this.reconnectAttempts = 0;
      const sub = {
        jsonrpc: '2.0',
        id: 1,
        method: 'logsSubscribe',
        params: [
          { mentions: [PUMP_PROGRAM_ID] },
          { commitment: 'confirmed' },
        ],
      };
      ws.send(JSON.stringify(sub));
    };

    ws.onmessage = (ev) => {
      const event = parseLaunchNotification(String(ev.data));
      if (event) this.onLaunch(event);
    };

    ws.onclose = () => {
      this.scheduleReconnect();
    };

    ws.onerror = (err) => {
      console.warn('[PumpLaunchMonitor] WebSocket error:', err);
      ws.close(); // triggers onclose → scheduleReconnect
    };
  }
}

/** Parse Helius logsNotification for Pump.fun Create instruction. */
export function parseLaunchNotification(text: string): PumpLaunchEvent | null {
  try {
    const v = JSON.parse(text) as Record<string, unknown>;
    const result = (v.params as Record<string, unknown>)?.result as Record<string, unknown> | undefined;
    const value = result?.value as Record<string, unknown> | undefined;
    if (!value) return null;

    const logs = value.logs as string[] | undefined;
    if (!logs?.some((l) => l.includes('Instruction: Create'))) return null;

    const signature = String(value.signature ?? '');
    const slot = Number((value.context as Record<string, unknown>)?.slot ?? 0);

    const mint = extractMintFromLogs(logs) ?? extractMintFromSignature(signature);
    if (!mint) return null;

    return {
      mint,
      signature,
      slot,
      detectedAtMs: Date.now(),
    };
  } catch {
    return null;
  }
}

function extractMintFromLogs(logs: string[]): string | undefined {
  for (const line of logs) {
    const m = line.match(/mint[:\s]+([1-9A-HJ-NP-Za-km-z]{32,44})/i);
    if (m?.[1]) return m[1];
  }
  return undefined;
}

function extractMintFromSignature(_sig: string): string | undefined {
  return undefined;
}

/** Build Helius WebSocket URL from API key. */
export function heliusWsUrl(apiKey: string): string {
  return `wss://mainnet.helius-rpc.com/?api-key=${apiKey}`;
}
