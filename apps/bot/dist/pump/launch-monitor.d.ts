export interface PumpLaunchEvent {
    mint: string;
    signature: string;
    slot: number;
    creator?: string;
    detectedAtMs: number;
}
type LaunchHandler = (event: PumpLaunchEvent) => void;
export declare class PumpLaunchMonitor {
    private readonly heliusWsUrl;
    private readonly onLaunch;
    private ws;
    private reconnectTimer;
    private running;
    private reconnectAttempts;
    constructor(heliusWsUrl: string, onLaunch: LaunchHandler);
    start(): void;
    stop(): void;
    private scheduleReconnect;
    private connect;
}
/** Parse Helius logsNotification for Pump.fun Create instruction. */
export declare function parseLaunchNotification(text: string): PumpLaunchEvent | null;
/** Build Helius WebSocket URL from API key. */
export declare function heliusWsUrl(apiKey: string): string;
export {};
