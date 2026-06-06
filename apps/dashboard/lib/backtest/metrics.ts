import type { BotStrategy, BotStrategyConfig } from '@/stores/botStore';

import type { StrategyRankingRow } from './applySelection';
import {
  simulateStrategyConfig,
  type BacktestReportSlice,
  type SimulatedBacktestResult,
} from './simulateStrategy';

/** Paper desk notional used for ROI % (matches ~10 SOL @ ~$250). */
export const PAPER_NOTIONAL_USD = 2500;

export interface RoiMetrics {
  windowRoiPct: number;
  dailyRoiPct: number;
  monthlyRoiPct: number;
}

export type EnrichedBacktestMetrics = SimulatedBacktestResult & RoiMetrics & {
  matchingRankingId: string | null;
};

let reportCache: BacktestReportSlice | null | undefined;

export function confidenceTone(score: number): 'green' | 'blue' | 'amber' | 'muted' {
  if (score >= 85) return 'green';
  if (score >= 70) return 'blue';
  if (score >= 55) return 'amber';
  return 'muted';
}

export function computeRoiMetrics(
  netPnlUsd: number,
  hours: number,
  notionalUsd = PAPER_NOTIONAL_USD,
): RoiMetrics {
  if (notionalUsd <= 0 || hours <= 0) {
    return { windowRoiPct: 0, dailyRoiPct: 0, monthlyRoiPct: 0 };
  }
  const windowRoiPct = (netPnlUsd / notionalUsd) * 100;
  const dailyRoiPct = windowRoiPct * (24 / hours);
  const monthlyRoiPct = dailyRoiPct * 30;
  return { windowRoiPct, dailyRoiPct, monthlyRoiPct };
}

export function formatRoiPct(pct: number, digits = 2): string {
  const sign = pct >= 0 ? '+' : '';
  return `${sign}${pct.toFixed(digits)}%`;
}

export function rankingForConfig(
  config: BotStrategyConfig,
  report: BacktestReportSlice | null,
): StrategyRankingRow | null {
  const rankings = report?.strategy_rankings as StrategyRankingRow[] | undefined;
  if (!rankings?.length) return null;

  const enabled = new Set<BotStrategy>();
  if (config.scalp) enabled.add('scalp');
  if (config.arb) enabled.add('arb');
  if (config.whale_copy) enabled.add('whale_copy');
  if (config.momentum) enabled.add('momentum');
  if (config.sniper) enabled.add('sniper');

  const exact = rankings.find((r) => {
    const keys = new Set(r.bot_store_keys);
    if (keys.size !== enabled.size) return false;
    return Array.from(enabled).every((k) => keys.has(k));
  });
  if (exact) return exact;

  let best = rankings[0];
  let bestScore = -1;
  for (const row of rankings) {
    const overlap = row.bot_store_keys.filter((k) => enabled.has(k as BotStrategy)).length;
    const score = overlap - Math.abs(row.bot_store_keys.length - enabled.size) * 0.25;
    if (score > bestScore) {
      bestScore = score;
      best = row;
    }
  }
  return best ?? null;
}

export function metricsFromRanking(
  row: StrategyRankingRow,
  hours = 6,
  notionalUsd = PAPER_NOTIONAL_USD,
): EnrichedBacktestMetrics {
  const roi = computeRoiMetrics(row.net_pnl_usd, hours, notionalUsd);
  return {
    winRate: row.win_probability,
    passProbability: row.pass_probability,
    expectedUsdPerTrade: row.expected_usd_per_trade,
    netPnlUsd: row.net_pnl_usd,
    sharpeApprox: row.sharpe_approx,
    tradesPerDay: row.trades_per_day,
    maxDrawdownUsd: row.max_drawdown_usd,
    confidenceScore: row.confidence_score,
    durationHours: hours,
    enabledEngines: row.bot_store_keys,
    notes: [`Holdout ranking #${row.rank}: ${row.name}.`],
    usedHoldoutData: true,
    matchingRankingId: row.id,
    ...roi,
  };
}

export function enrichSimulation(
  result: SimulatedBacktestResult,
  report: BacktestReportSlice | null,
  config: BotStrategyConfig,
  notionalUsd = PAPER_NOTIONAL_USD,
): EnrichedBacktestMetrics {
  const ranking = rankingForConfig(config, report);
  const roi = computeRoiMetrics(result.netPnlUsd, result.durationHours, notionalUsd);
  return {
    ...result,
    ...roi,
    matchingRankingId: ranking?.id ?? null,
  };
}

export function simulateEnriched(
  config: BotStrategyConfig,
  report: BacktestReportSlice | null,
  hours = 6,
  notionalUsd = PAPER_NOTIONAL_USD,
): EnrichedBacktestMetrics {
  const raw = simulateStrategyConfig(config, report, hours);
  return enrichSimulation(raw, report, config, notionalUsd);
}

export function metricsForEngine(
  engine: BotStrategy,
  config: BotStrategyConfig,
  report: BacktestReportSlice | null,
  hours = 6,
): EnrichedBacktestMetrics {
  const solo: BotStrategyConfig = {
    ...config,
    scalp: engine === 'scalp',
    arb: engine === 'arb',
    whale_copy: engine === 'whale_copy',
    momentum: engine === 'momentum',
    sniper: engine === 'sniper',
  };
  return simulateEnriched(solo, report, hours);
}

export async function fetchBacktestReport(): Promise<BacktestReportSlice | null> {
  if (reportCache) return reportCache;
  try {
    const res = await fetch('/data/backtest_results.json');
    if (!res.ok) return null;
    const data = (await res.json()) as BacktestReportSlice;
    reportCache = data;
    return data;
  } catch {
    return null;
  }
}

export function clearBacktestReportCache(): void {
  reportCache = undefined;
}

export function backtestHoursFromReport(report: BacktestReportSlice | null): number {
  const h = report?.baseline?.metrics?.duration_hours;
  return typeof h === 'number' && h > 0 ? h : 6;
}
