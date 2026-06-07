/**
 * Lightweight Express monitoring dashboard.
 *
 * GET /status  — uptime, wallet balance, totals
 * GET /trades  — last 50 trades from SQLite
 * GET /health  — 200 if running, 503 if halted
 */
import express from 'express';
import { Connection, PublicKey } from '@solana/web3.js';
import { getRecentTrades, getTotalPnlUsd, getTradeSummary } from '../db/sqlite.js';
import { logger } from '../logger.js';
import { computeAnalytics } from '../analytics/metrics.js';
import { buildDashboardHtml } from './dashboard.js';
import { runBacktest } from '../backtest/engine.js';
import { runAutoTuner } from '../analytics/auto-tuner.js';
const STARTED_AT = Date.now();
export function startMonitoringServer(opts = {}) {
    const port = opts.port ?? Number(process.env.MONITOR_PORT ?? '3333');
    const app = express();
    const connection = opts.rpcUrl
        ? new Connection(opts.rpcUrl, 'confirmed')
        : null;
    const walletPubkey = opts.walletPublicKey ?? process.env.BOT_WALLET_PUBKEY;
    // ── GET / ─────────────────────────────────────────────────────────────────
    // Human-readable HTML dashboard with auto-refresh.
    app.get('/', (_req, res) => {
        res.setHeader('Content-Type', 'text/html; charset=utf-8');
        res.send(buildDashboardHtml());
    });
    // ── GET /health ────────────────────────────────────────────────────────────
    app.get('/health', (_req, res) => {
        const healthy = opts.isHealthy ? opts.isHealthy() : true;
        res.status(healthy ? 200 : 503).json({
            status: healthy ? 'running' : 'halted',
            uptimeMs: Date.now() - STARTED_AT,
        });
    });
    // ── GET /status ────────────────────────────────────────────────────────────
    app.get('/status', async (_req, res) => {
        try {
            const summary = getTradeSummary();
            const totalPnlUsd = getTotalPnlUsd();
            let walletBalanceSol = null;
            if (connection && walletPubkey) {
                try {
                    const lamports = await connection.getBalance(new PublicKey(walletPubkey), 'confirmed');
                    walletBalanceSol = lamports / 1_000_000_000;
                }
                catch {
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
        }
        catch (err) {
            logger.error({ err }, 'GET /status error');
            res.status(500).json({ error: 'internal_error' });
        }
    });
    // ── GET /trades ────────────────────────────────────────────────────────────
    app.get('/trades', (req, res) => {
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
        }
        catch (err) {
            logger.error({ err }, 'GET /trades error');
            res.status(500).json({ error: 'internal_error' });
        }
    });
    // ── GET /journal ───────────────────────────────────────────────────────────
    app.get('/journal', (req, res) => {
        if (!opts.journal) {
            res.json({ events: [] });
            return;
        }
        const type = req.query['type'];
        const limit = Math.min(Number(req.query['limit'] ?? '100'), 500);
        const events = type
            ? opts.journal.byType(type)
            : [...opts.journal.all()];
        res.json({ count: events.length, events: events.slice(-limit) });
    });
    // ── GET /scan-stats ────────────────────────────────────────────────────────
    app.get('/scan-stats', (_req, res) => {
        const stats = opts.getStats?.() ?? null;
        res.json(stats ?? { error: 'stats_unavailable' });
    });
    // ── GET /analytics ─────────────────────────────────────────────────────────
    // Full session analytics: hit rate, avg profit, rejection breakdown, per-pair win rates.
    app.get('/analytics', (_req, res) => {
        if (!opts.journal) {
            res.json({ error: 'journal_unavailable' });
            return;
        }
        const events = [...opts.journal.all()];
        const snap = computeAnalytics(events);
        // Extra fields useful for post-session review
        const executions = events.filter((e) => e.type === 'execution');
        const realized = executions.map((e) => e.realizedProfitUsd ?? 0).filter(Number.isFinite);
        const sessionPnl = realized.reduce((a, b) => a + b, 0);
        const uptimeSec = (Date.now() - STARTED_AT) / 1000;
        const tradesPerHour = executions.length / (uptimeSec / 3600);
        // Profit distribution buckets
        const buckets = {
            'loss': realized.filter((v) => v < 0).length,
            '$0-$0.10': realized.filter((v) => v >= 0 && v < 0.10).length,
            '$0.10-$0.25': realized.filter((v) => v >= 0.10 && v < 0.25).length,
            '$0.25-$0.50': realized.filter((v) => v >= 0.25 && v < 0.50).length,
            '$0.50-$1': realized.filter((v) => v >= 0.50 && v < 1.00).length,
            '$1+': realized.filter((v) => v >= 1.00).length,
        };
        const capital = opts.getCapital?.() ?? null;
        res.json({
            ...snap,
            sessionPnlUsd: Number(sessionPnl.toFixed(6)),
            tradesPerHour: Number(tradesPerHour.toFixed(1)),
            uptimeMinutes: Number((uptimeSec / 60).toFixed(1)),
            profitDistribution: buckets,
            // Capital stage — populated when adaptive arb is active
            capitalSol: capital?.capitalSol ?? null,
            capitalStage: capital?.stage ?? null,
        });
    });
    // ── GET /backtest ──────────────────────────────────────────────────────────
    // Parameter sweep backtest on the current session's spread observations.
    app.get('/backtest', (req, res) => {
        if (!opts.journal) {
            res.json({ error: 'journal_unavailable' });
            return;
        }
        const events = [...opts.journal.all()];
        const solPrice = Number(req.query['solPrice'] ?? '150');
        const tradeSizeUi = Number(req.query['tradeSizeUi'] ?? '0.5');
        const uptimeSec = (Date.now() - STARTED_AT) / 1000;
        const hoursOfData = uptimeSec / 3600;
        const sweep = runBacktest(events, tradeSizeUi, solPrice, hoursOfData);
        const tuner = runAutoTuner(events, solPrice);
        res.json({
            sweep: {
                breakEvenBps: sweep.breakEvenBps,
                thresholdGapBps: sweep.thresholdGapBps,
                totalObservations: sweep.observations.length,
                recommendations: sweep.recommendations,
                topConfigs: sweep.configs.slice(0, 5).map((c) => ({
                    minSpreadBps: c.config.minSpreadBps,
                    minProfitUsd: c.config.minProfitUsd,
                    triggeredTrades: c.triggeredTrades,
                    triggerRatePct: (c.triggerRate * 100).toFixed(1) + '%',
                    estimatedTotalProfitUsd: Number(c.estimatedTotalProfitUsd.toFixed(4)),
                    avgNetProfitPerTrade: Number(c.avgNetProfitPerTrade.toFixed(4)),
                    estimatedAprPct: Number(c.estimatedAprPct.toFixed(1)),
                })),
            },
            autoTuner: tuner,
        });
    });
    const server = app.listen(port, () => {
        logger.info({ port }, 'monitoring server started');
    });
    return () => {
        server.close();
        logger.info('monitoring server stopped');
    };
}
