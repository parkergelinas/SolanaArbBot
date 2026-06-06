/** Jupiter Recurring API client stub — DCA / treasury automation (not wired yet). */
import type { BotEnv } from '../config/env.js';
export type RecurringFrequency = 'hourly' | 'daily' | 'weekly';
export interface RecurringOrderRequest {
    inputMint: string;
    outputMint: string;
    amountPerCycle: string;
    frequency: RecurringFrequency;
    slippageBps?: number;
}
export interface RecurringOrderResponse {
    orderId: string;
    status: 'active' | 'paused';
}
/**
 * Stub client for Jupiter Recurring API.
 * Live wiring deferred — enable via `BOT_ENABLE_RECURRING_API=1` when ready.
 */
export declare class JupiterRecurringClient {
    private readonly env;
    private readonly baseUrl;
    constructor(env: BotEnv, baseUrl?: string);
    private headers;
    createOrder(_req: RecurringOrderRequest): Promise<RecurringOrderResponse>;
    pauseOrder(_orderId: string): Promise<void>;
    buildCreateUrl(): string;
}
