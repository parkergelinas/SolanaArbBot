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
export declare class PumpLaunchMonitor {
    private readonly heliusWsUrl;
    private readonly onLaunch;
    private ws;
    private reconnectTimer;
    private running;
    constructor(heliusWsUrl: string, onLaunch: LaunchHandler);
    start(): void;
    stop(): void;
    private connect;
}
/** Parse Helius logsNotification for Pump.fun Create instruction. */
export declare function parseLaunchNotification(text: string): PumpLaunchEvent | null;
/** Build Helius WebSocket URL from API key. */
export declare function heliusWsUrl(apiKey: string): string;
export {};
