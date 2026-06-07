/**
 * Lightweight Express monitoring dashboard.
 *
 * GET /status  — uptime, wallet balance, totals
 * GET /trades  — last 50 trades from SQLite
 * GET /health  — 200 if running, 503 if halted
 */
import type { TradeJournal } from '../state/journal.js';
import type { EngineStats } from '../app/engine.js';
export interface MonitoringServerOptions {
    port?: number;
    rpcUrl?: string;
    walletPublicKey?: string;
    /** Returns true if the bot is currently running and not halted. */
    isHealthy?: () => boolean;
    /** Live journal for /journal endpoint. */
    journal?: TradeJournal;
    /** Live engine stats for /scan-stats endpoint. */
    getStats?: () => EngineStats;
}
export declare function startMonitoringServer(opts?: MonitoringServerOptions): () => void;
