/**
 * Multi-strategy backtest runner with auto-advance on failure.
 *
 * Runs each strategy in sequence for a fixed window.  If a strategy does
 * not meet profitability criteria, it is marked "failed" and the runner
 * advances to the next strategy automatically.  Profitable strategies are
 * promoted to a "passed" list for live deployment consideration.
 *
 * Usage (environment variables):
 *   BACKTEST_DURATION_HR=3   — session window per strategy (default: 0.33 hr / 20 min)
 *   BACKTEST_MIN_PROFIT=0.05 — min net profit USD threshold
 *   BOT_SOL_PRICE=150        — SOL price for notional calculations
 *   JUPITER_API_KEY=...      — optional Jupiter API key
 *
 * Run:
 *   BOT_PAPER_MODE=1 BOT_LIVE_QUOTES=1 npx tsx src/backtest/run-backtest.ts
 */

import { writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { JupiterClient } from '../jupiter/client.js';
import { loadEnv } from '../config/env.js';

import type { ScannableStrategy } from '../signals/types.js';
import {
  DEFAULT_SESSION_CONFIG,
  DEFAULT_ACCEPTANCE,
  runStrategySession,
  evaluateSession,
  type SessionConfig,
  type AcceptanceCriteria,
  type AcceptanceResult,
} from './strategy-session.js';

// ── Strategy imports ──────────────────────────────────────────────────────────
import { LstSpreadArbStrategy, DEFAULT_LST_CONFIG } from '../strategy/lst-spread-arb.js';
import { TriangleArbStrategy, DEFAULT_TRIANGLE_CONFIG } from '../strategy/triangle-arb.js';
import { StableDepegStrategy, DEFAULT_STABLE_CONFIG } from '../strategy/stable-depeg.js';
import { MomentumLagStrategy, DEFAULT_MOMENTUM_CONFIG } from '../strategy/momentum-lag.js';
import { RouteDiversitySweepStrategy, DEFAULT_DIVERSITY_CONFIG } from '../strategy/route-diversity-sweep.js';

// ── Existing strategies for comparison ───────────────────────────────────────
import { RoundTripQuoteArbStrategy, DEFAULT_ROUND_TRIP_CONFIG } from '../strategy/round-trip-arb.js';

export interface StrategyRunResult {
  strategyId: string;
  strategyName: string;
  sessionResult: AcceptanceResult;
  /** ISO timestamp when this run started. */
  startedAt: string;
  /** ISO timestamp when this run ended. */
  endedAt: string;
}

export interface MultiStrategyReport {
  generatedAt: string;
  totalStrategiesTested: number;
  passedStrategies: StrategyRunResult[];
  failedStrategies: StrategyRunResult[];
  sessionConfig: SessionConfig;
  acceptance: AcceptanceCriteria;
  /** Ranked list by totalNetProfitUsd descending. */
  rankings: Array<{
    rank: number;
    strategyId: string;
    totalNetProfitUsd: number;
    tradesPerHour: number;
    winRate: number;
    avgNetProfitUsd: number;
    passed: boolean;
  }>;
}

function formatDuration(ms: number): string {
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return `${m}m ${rem}s`;
}

function log(msg: string): void {
  const ts = new Date().toISOString().slice(11, 19);
  console.log(`[${ts}] ${msg}`);
}

function buildStrategies(client: JupiterClient): Array<{ name: string; strategy: ScannableStrategy }> {
  return [
    // New strategies (the ones we're testing)
    { name: 'LST Spread Arb',        strategy: new LstSpreadArbStrategy(client, DEFAULT_LST_CONFIG) },
    { name: 'Triangle Arb',          strategy: new TriangleArbStrategy(client, DEFAULT_TRIANGLE_CONFIG) },
    { name: 'Stablecoin Depeg',      strategy: new StableDepegStrategy(client, DEFAULT_STABLE_CONFIG) },
    { name: 'Momentum Lag Capture',  strategy: new MomentumLagStrategy(client, DEFAULT_MOMENTUM_CONFIG) },
    { name: 'Route Diversity Sweep', strategy: new RouteDiversitySweepStrategy(client, DEFAULT_DIVERSITY_CONFIG) },
    // Baseline comparison
    { name: 'Round-Trip Quote Arb (baseline)', strategy: new RoundTripQuoteArbStrategy(client, 0.5, null, DEFAULT_ROUND_TRIP_CONFIG) },
  ];
}

export async function runMultiStrategyBacktest(opts: {
  durationHrPerStrategy?: number;
  minProfitUsd?: number;
  solPriceUsd?: number;
  outputDir?: string;
  stopOnFirstPass?: boolean;
} = {}): Promise<MultiStrategyReport> {
  const env = loadEnv();
  const client = new JupiterClient(env);

  const durationHr = opts.durationHrPerStrategy
    ?? Number(process.env['BACKTEST_DURATION_HR'] ?? '0.33');
  const durationMs = Math.round(durationHr * 3_600_000);
  const solPriceUsd = opts.solPriceUsd
    ?? Number(process.env['BOT_SOL_PRICE'] ?? '150');
  const minProfitUsd = opts.minProfitUsd
    ?? Number(process.env['BACKTEST_MIN_PROFIT'] ?? '0.01');

  const sessionCfg: SessionConfig = {
    ...DEFAULT_SESSION_CONFIG,
    durationMs,
    solPriceUsd,
    minProfitUsd,
    scanIntervalMs: 12_000,  // scan every 12 s
  };

  const acceptance: AcceptanceCriteria = {
    ...DEFAULT_ACCEPTANCE,
    minTotalNetProfitUsd: minProfitUsd * 2,
  };

  const strategies = buildStrategies(client);
  const results: StrategyRunResult[] = [];

  log(`=== Multi-Strategy Backtest ===`);
  log(`Strategies: ${strategies.length}`);
  log(`Session duration: ${formatDuration(durationMs)} per strategy`);
  log(`Min profit threshold: $${minProfitUsd}`);
  log(`SOL price: $${solPriceUsd}`);
  log('');

  for (const { name, strategy } of strategies) {
    log(`▶ Starting: ${name} (${strategy.id})`);
    log(`  Duration: ${formatDuration(durationMs)}`);

    const startedAt = new Date().toISOString();

    const metrics = await runStrategySession(
      strategy,
      client,
      sessionCfg,
      (elapsed, prog) => {
        const pct = ((elapsed / durationMs) * 100).toFixed(0);
        log(
          `  [${pct}%] trades=${prog.actionableDecisions} ` +
          `pnl=$${prog.totalNetProfitUsd.toFixed(4)} ` +
          `tph=${prog.tradesPerHour.toFixed(1)}`,
        );
      },
    );

    const endedAt = new Date().toISOString();
    const sessionResult = evaluateSession(metrics, acceptance);

    results.push({ strategyId: strategy.id, strategyName: name, sessionResult, startedAt, endedAt });

    if (sessionResult.passed) {
      log(`  ✅ PASSED — ${name}`);
      log(`     trades/hr: ${metrics.tradesPerHour.toFixed(1)}`);
      log(`     win rate:  ${(metrics.winRate * 100).toFixed(1)}%`);
      log(`     total P&L: $${metrics.totalNetProfitUsd.toFixed(4)}`);
      log(`     avg/trade: $${metrics.avgNetProfitUsd.toFixed(5)}`);
    } else {
      log(`  ❌ FAILED — ${name}`);
      log(`     reasons: ${sessionResult.failReasons.join(', ')}`);
      log(`     trades/hr: ${metrics.tradesPerHour.toFixed(1)}`);
      log(`     win rate:  ${(metrics.winRate * 100).toFixed(1)}%`);
      log(`     total P&L: $${metrics.totalNetProfitUsd.toFixed(4)}`);
    }

    if (metrics.errors.length > 0) {
      log(`     errors (${metrics.errors.length}): ${metrics.errors.slice(0, 3).join('; ')}`);
    }

    log('');

    if (opts.stopOnFirstPass && sessionResult.passed) {
      log('stopOnFirstPass=true — stopping after first pass.');
      break;
    }
  }

  const passed = results.filter((r) => r.sessionResult.passed);
  const failed = results.filter((r) => !r.sessionResult.passed);

  // Build rankings
  const rankings = results
    .map((r) => ({
      strategyId: r.strategyId,
      totalNetProfitUsd: r.sessionResult.metrics.totalNetProfitUsd,
      tradesPerHour: r.sessionResult.metrics.tradesPerHour,
      winRate: r.sessionResult.metrics.winRate,
      avgNetProfitUsd: r.sessionResult.metrics.avgNetProfitUsd,
      passed: r.sessionResult.passed,
    }))
    .sort((a, b) => b.totalNetProfitUsd - a.totalNetProfitUsd)
    .map((r, i) => ({ rank: i + 1, ...r }));

  const report: MultiStrategyReport = {
    generatedAt: new Date().toISOString(),
    totalStrategiesTested: results.length,
    passedStrategies: passed,
    failedStrategies: failed,
    sessionConfig: sessionCfg,
    acceptance,
    rankings,
  };

  // ── Save report ─────────────────────────────────────────────────────────────
  const outDir = opts.outputDir ?? resolve(
    dirname(fileURLToPath(import.meta.url)),
    '../../../../data',
  );
  try {
    mkdirSync(outDir, { recursive: true });
    const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    const outPath = resolve(outDir, `strategy_backtest_${ts}.json`);
    writeFileSync(outPath, JSON.stringify(report, null, 2), 'utf-8');
    log(`Report saved to: ${outPath}`);
  } catch (err) {
    log(`Warning: could not save report — ${err instanceof Error ? err.message : err}`);
  }

  // ── Summary ─────────────────────────────────────────────────────────────────
  log('=== Summary ===');
  log(`Tested: ${results.length}  Passed: ${passed.length}  Failed: ${failed.length}`);
  log('');
  log('Rankings:');
  for (const r of rankings) {
    const badge = r.passed ? '✅' : '❌';
    log(
      `  ${r.rank}. ${badge} ${r.strategyId.padEnd(30)} ` +
      `P&L=$${r.totalNetProfitUsd.toFixed(4).padStart(8)}  ` +
      `tph=${r.tradesPerHour.toFixed(1).padStart(5)}  ` +
      `wr=${(r.winRate * 100).toFixed(0).padStart(3)}%`,
    );
  }

  return report;
}
