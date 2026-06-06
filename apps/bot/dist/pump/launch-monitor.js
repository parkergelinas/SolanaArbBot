import { PUMP_PROGRAM_ID } from './constants.js';
/**
 * Monitor Pump.fun `Create` instructions via Helius logsSubscribe.
 * Docs: pump-public-docs — `create(user, name, symbol, uri, creator)`.
 */
export class PumpLaunchMonitor {
    heliusWsUrl;
    onLaunch;
    ws = null;
    reconnectTimer = null;
    running = false;
    constructor(heliusWsUrl, onLaunch) {
        this.heliusWsUrl = heliusWsUrl;
        this.onLaunch = onLaunch;
    }
    start() {
        if (this.running)
            return;
        this.running = true;
        this.connect();
    }
    stop() {
        this.running = false;
        if (this.reconnectTimer)
            clearTimeout(this.reconnectTimer);
        this.ws?.close();
        this.ws = null;
    }
    connect() {
        if (!this.running)
            return;
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
            if (event)
                this.onLaunch(event);
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
export function parseLaunchNotification(text) {
    try {
        const v = JSON.parse(text);
        const result = v.params?.result;
        const value = result?.value;
        if (!value)
            return null;
        const logs = value.logs;
        if (!logs?.some((l) => l.includes('Instruction: Create')))
            return null;
        const signature = String(value.signature ?? '');
        const slot = Number(value.context?.slot ?? 0);
        const mint = extractMintFromLogs(logs) ?? extractMintFromSignature(signature);
        if (!mint)
            return null;
        return {
            mint,
            signature,
            slot,
            detectedAtMs: Date.now(),
        };
    }
    catch {
        return null;
    }
}
function extractMintFromLogs(logs) {
    for (const line of logs) {
        const m = line.match(/mint[:\s]+([1-9A-HJ-NP-Za-km-z]{32,44})/i);
        if (m?.[1])
            return m[1];
    }
    return undefined;
}
function extractMintFromSignature(_sig) {
    return undefined;
}
/** Build Helius WebSocket URL from API key. */
export function heliusWsUrl(apiKey) {
    return `wss://mainnet.helius-rpc.com/?api-key=${apiKey}`;
}
