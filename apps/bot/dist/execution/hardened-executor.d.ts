/**
 * Production-grade swap executor:
 * - Versioned (v0) transactions with address lookup tables
 * - Dynamic compute-unit estimation via simulation
 * - Simulate-before-send; aborts on simulation failure
 * - Dead-man's switch: halts after 3 consecutive failures
 * - Jito bundle submission with SOL-transfer tip; falls back to standard RPC
 * - Every attempt persisted to SQLite with full metadata
 */
import type { BotEnv } from '../config/env.js';
import type { JupiterClient } from '../jupiter/client.js';
import type { TradeJournal } from '../state/journal.js';
import type { TradeDecision } from '../strategy/types.js';
import { DeadManSwitch } from './dead-man-switch.js';
import type { ExecutionResult } from './live-executor.js';
export interface HardenedExecutorOptions {
    maxRetries?: number;
    baseBackoffMs?: number;
    jitoEnabled?: boolean;
    jitoTipLamports?: number;
    minProfitLamports?: number;
}
export declare class HardenedExecutor {
    private readonly env;
    private readonly client;
    private readonly journal;
    readonly deadManSwitch: DeadManSwitch;
    private readonly connection;
    private readonly jito;
    private readonly keypair;
    private readonly jitoEnabled;
    private readonly jitoTipLamports;
    private readonly minProfitLamports;
    private jitoAvailable;
    constructor(env: BotEnv, client: JupiterClient, journal: TradeJournal, opts?: HardenedExecutorOptions);
    execute(decision: TradeDecision, opts?: {
        maxRetries?: number;
        baseBackoffMs?: number;
    }): Promise<ExecutionResult>;
    private attemptOnce;
    /** Rebuild VersionedTransaction with correct CU limit and priority fee, sign it (no Jito tip). */
    private rebuildWithCuLimit;
    /** Returns p75 of recent slot prioritization fees in micro-lamports per compute unit. */
    private estimatePriorityFee;
    private paperResult;
    private logToJournal;
    private persistToDb;
}
