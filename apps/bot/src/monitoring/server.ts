/**
 * Lightweight Express monitoring dashboard.
 *
 * GET /status  — uptime, wallet balance, totals
 * GET /trades  — last 50 trades from SQLite
 * GET /health  — 200 if running, 503 if halted
 */

import express, { type Request, type Response } from 'express';
import { Connection, PublicKey } from '@solana/web3.js';
import { getRecentTrades, getTotalPnlUsd, getTradeSummary } from '../db/sqlite.js';
import { logger } from '../logger.js';
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

const STARTED_AT = Date.now();

export function startMonitoringServer(opts: MonitoringServerOptions = {}): () => void {
  const port = opts.port ?? Number(process.env.MONITOR_PORT ?? '3333');
  const app = express();

  const connection = opts.rpcUrl
    ? new Connection(opts.rpcUrl, 'confirmed')
    : null;
  const walletPubkey = opts.walletPublicKey ?? process.env.BOT_WALLET_PUBKEY;

  // ── GET /health ────────────────────────────────────────────────────────────
  app.get('/health', (_req: Request, res: Response) => {
    const healthy = opts.isHealthy ? opts.isHealthy() : true;
    res.status(healthy ? 200 : 503).json({
      status: healthy ? 'running' : 'halted',
      uptimeMs: Date.now() - STARTED_AT,
    });
  });

  // ── GET /status ────────────────────────────────────────────────────────────
  app.get('/status', async (_req: Request, res: Response) => {
    try {
      const summary = getTradeSummary();
      const totalPnlUsd = getTotalPnlUsd();

      let walletBalanceSol: number | null = null;
      if (connection && walletPubkey) {
        try {
          const lamports = await connection.getBalance(
            new PublicKey(walletPubkey),
            'confirmed',
          );
          walletBalanceSol = lamports / 1_000_000_000;
        } catch {
          // Non-fatal — return null balance
        }
      }

      res.json({
        status: opts.isHealthy?.() !== false ? 'running' : 'halted',
        uptimeMs: Date.now() - STARTED_AT,
        walletBalanceSol,
        totalTrades: summary.total,
        successfulTrades: summary.successful,
        failedTrades: summary.failed,
        totalPnlUsd: Number(totalPnlUsd.toFixed(6)),
      });
    } catch (err) {
      logger.error({ err }, 'GET /status error');
      res.status(500).json({ error: 'internal_error' });
    }
  });

  // ── GET /trades ────────────────────────────────────────────────────────────
  app.get('/trades', (req: Request, res: Response) => {
    try {
      const limit = Math.min(Number(req.query['limit'] ?? '50'), 200);
      const trades = getRecentTrades(limit);

      const enriched = trades.map((t) => ({
        ...t,
        profit_breakdown: {
          profit_usd: t.profit_usd,
          priority_fee_lamports: t.priority_fee_lamports,
          jito_tip_lamports: t.jito_tip_lamports,
          compute_units_used: t.compute_units_used,
        },
      }));

      res.json({ count: enriched.length, trades: enriched });
    } catch (err) {
      logger.error({ err }, 'GET /trades error');
      res.status(500).json({ error: 'internal_error' });
    }
  });

  // ── GET /journal ───────────────────────────────────────────────────────────
  app.get('/journal', (req: Request, res: Response) => {
    if (!opts.journal) { res.json({ events: [] }); return; }
    const type = req.query['type'] as string | undefined;
    const limit = Math.min(Number(req.query['limit'] ?? '100'), 500);
    const events = type
      ? opts.journal.byType(type as Parameters<TradeJournal['byType']>[0])
      : [...opts.journal.all()];
    res.json({ count: events.length, events: events.slice(-limit) });
  });

  // ── GET /scan-stats ────────────────────────────────────────────────────────
  app.get('/scan-stats', (_req: Request, res: Response) => {
    const stats = opts.getStats?.() ?? null;
    res.json(stats ?? { error: 'stats_unavailable' });
  });

  const server = app.listen(port, () => {
    logger.info({ port }, 'monitoring server started');
  });

  return () => {
    server.close();
    logger.info('monitoring server stopped');
  };
}
