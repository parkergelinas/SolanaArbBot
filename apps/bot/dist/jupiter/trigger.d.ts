/** Jupiter Trigger API client stub — TP/SL/breakout orders (not wired to engine yet). */
import type { BotEnv } from '../config/env.js';
export type TriggerOrderKind = 'take_profit' | 'stop_loss' | 'breakout';
export interface TriggerOrderRequest {
    inputMint: string;
    outputMint: string;
    amount: string;
    kind: TriggerOrderKind;
    triggerPriceUsd: number;
    slippageBps?: number;
}
export interface TriggerOrderResponse {
    orderId: string;
    status: 'pending' | 'active';
}
/**
 * Stub client for Jupiter Trigger API.
 * Live wiring deferred — enable via `BOT_ENABLE_TRIGGER_API=1` when ready.
 */
export declare class JupiterTriggerClient {
    private readonly env;
    private readonly baseUrl;
    constructor(env: BotEnv, baseUrl?: string);
    private headers;
    /** Create a trigger order — throws until live integration is implemented. */
    createOrder(_req: TriggerOrderRequest): Promise<TriggerOrderResponse>;
    /** Cancel an existing trigger order — stub. */
    cancelOrder(_orderId: string): Promise<void>;
    buildCreateUrl(): string;
}
