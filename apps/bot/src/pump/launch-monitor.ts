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
export class PumpLaunchMonitor {
  private ws: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private running = false;

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

  private connect(): void {
    if (!this.running) return;

    const ws = new WebSocket(this.heliusWsUrl);
    this.ws = ws;

    ws.onopen = () => {
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
      if (this.running) {
        this.reconnectTimer = setTimeout(() => this.connect(), 5000);
      }
    };

    ws.onerror = () => {
      ws.close();
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
