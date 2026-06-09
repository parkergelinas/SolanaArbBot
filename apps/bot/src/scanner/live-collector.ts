import { fileURLToPath } from 'url';
import type { Request, Response } from 'express';
import { logger } from '../logger.js';
import { ScannerEngine } from './scanner-engine.js';
import { SseServer } from './sse-server.js';
import {
  insertOpportunity,
  getRecentOpportunities,
  getOpportunitiesSince,
  getStats,
  closeOpportunityDb,
  type ScannedOpportunity,
} from './opportunity-store.js';

const SCANNER_PORT = Number(process.env.SCANNER_PORT ?? '4444');
const SCANNER_SCAN_INTERVAL_MS = Number(process.env.SCANNER_SCAN_INTERVAL_MS ?? '12000');
const SCANNER_CAPITAL_USD = Number(process.env.SCANNER_CAPITAL_USD ?? '150');

export interface ScannerDaemon {
  stop(): Promise<void>;
}

export async function startScannerDaemon(): Promise<ScannerDaemon> {
  logger.info(
    {
      port: SCANNER_PORT,
      intervalMs: SCANNER_SCAN_INTERVAL_MS,
      capitalUsd: SCANNER_CAPITAL_USD,
    },
    'scanner: starting',
  );

  const engine = new ScannerEngine(SCANNER_CAPITAL_USD);
  const sse = new SseServer();

  // ── REST endpoints ───────────────────────────────────────────────────────
  sse.app.get('/api/opportunities', (req: Request, res: Response) => {
    const limit = Math.min(Number(req.query['limit'] ?? '100'), 1000);
    res.json(getRecentOpportunities(limit));
  });

  sse.app.get('/api/stats', (_req: Request, res: Response) => {
    res.json({ ...getStats(), clientCount: sse.clientCount });
  });

  sse.app.get('/api/export', (req: Request, res: Response) => {
    const format = (req.query['format'] as string) ?? 'json';
    const since = req.query['since'] ? Number(req.query['since']) : undefined;
    const rows = since != null
      ? getOpportunitiesSince(Date.now() - since)
      : getRecentOpportunities(10_000);

    if (format === 'csv') {
      const header = 'id,timestamp,strategy_id,pair_label,spread_bps,net_profit_usd,score,sim_pass,survival_200ms,capital_usd\n';
      const body = rows
        .map(
          (r) =>
            `${r.id},${r.timestamp},${r.strategy_id},${r.pair_label},${r.spread_bps},` +
            `${r.net_profit_usd},${r.score},${r.sim_pass},${r.survival_200ms},${r.capital_usd}`,
        )
        .join('\n');
      res.setHeader('Content-Type', 'text/csv');
      res.setHeader('Content-Disposition', 'attachment; filename="opportunities.csv"');
      res.send(header + body);
    } else if (format === 'ndjson') {
      res.setHeader('Content-Type', 'application/x-ndjson');
      res.send(rows.map((r) => JSON.stringify(r)).join('\n'));
    } else {
      res.json(rows);
    }
  });

  // ── Init + start ─────────────────────────────────────────────────────────
  logger.info('scanner: initialising pair registry');
  await engine.init();

  sse.start(SCANNER_PORT, () => getRecentOpportunities(20));
  logger.info({ port: SCANNER_PORT }, 'scanner: HTTP server listening');

  // ── Scan loop ─────────────────────────────────────────────────────────────
  let scanning = false;
  const tick = setInterval(async () => {
    if (scanning) return;
    scanning = true;
    try {
      const results = await engine.scan();
      for (const r of results) {
        const opp: Omit<ScannedOpportunity, 'id'> = {
          timestamp: r.timestamp,
          strategy_id: r.strategyId,
          pair_label: r.pairLabel,
          spread_bps: r.spreadBps,
          net_profit_usd: r.netProfitUsd,
          score: r.score,
          sim_pass: r.simPass ? 1 : 0,
          survival_200ms: r.survival200ms ? 1 : 0,
          capital_usd: r.capitalUsd,
          raw_json: JSON.stringify(r.decision),
        };
        insertOpportunity(opp);
        sse.broadcast('opportunity', opp);
      }
      sse.broadcast('stats', { clientCount: sse.clientCount, total: getStats().total });
      if (results.length > 0) {
        logger.info({ count: results.length }, 'scanner: scan complete');
      }
    } catch (err) {
      logger.warn({ err }, 'scanner: scan tick error');
    } finally {
      scanning = false;
    }
  }, SCANNER_SCAN_INTERVAL_MS);

  return {
    async stop(): Promise<void> {
      clearInterval(tick);
      engine.stop();
      await sse.stop();
      closeOpportunityDb();
    },
  };
}

// ── Direct execution: npx tsx src/scanner/live-collector.ts ──────────────────
const isMain = fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  const daemon = await startScannerDaemon();
  const shutdown = async (signal: string): Promise<void> => {
    logger.info({ signal }, 'scanner: shutdown signal received');
    await daemon.stop();
    logger.info('scanner: clean exit');
    process.exit(0);
  };
  process.once('SIGTERM', () => void shutdown('SIGTERM'));
  process.once('SIGINT',  () => void shutdown('SIGINT'));
}
