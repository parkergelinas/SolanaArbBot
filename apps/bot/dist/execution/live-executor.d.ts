import { Connection } from '@solana/web3.js';
import type { BotEnv } from '../config/env.js';
import { JupiterClient } from '../jupiter/client.js';
import type { TradeDecision } from '../strategy/types.js';
import { TradeJournal } from '../state/journal.js';
export interface ExecutionResult {
    success: boolean;
    signature?: string;
    realizedProfitUsd?: number;
    error?: string;
    routeMetadata?: Record<string, unknown>;
}
export interface LiveExecutorOptions {
    maxRetries?: number;
    baseBackoffMs?: number;
    simulateBeforeSend?: boolean;
}
/** Live swap executor with retry, blockhash handling, and reconciliation hooks. */
export declare class LiveExecutor {
    private readonly env;
    private readonly client;
    private readonly journal;
    private connection;
    constructor(env: BotEnv, client: JupiterClient, journal: TradeJournal, opts?: {
        connection?: Connection;
    });
    execute(decision: TradeDecision, opts?: LiveExecutorOptions): Promise<ExecutionResult>;
}
