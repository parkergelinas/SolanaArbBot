import type { BotStrategyConfig } from '@/stores/botStore';
import { DEFAULT_STRATEGY_CONFIG } from '@/lib/strategies/presets';

export interface StrategyMetricSlice {
  strategy: string;
  total_trades: number;
  winning_trades: number;
  losing_trades: number;
  net_pnl_usd: number;
  win_rate: number;
  avg_net_per_trade_usd?: number;
  trades_per_day?: number;
  max_drawdown_usd?: number;
  sharpe_approx?: number;
}

export interface BacktestReportSlice {
  baseline?: {
    metrics?: {
      scalp?: StrategyMetricSlice;
      arb?: StrategyMetricSlice;
      combined_net_pnl_usd?: number;
      combined_trades_per_day?: number;
      combined_trades?: number;
      duration_hours?: number;
    };
  };
  retest?: {
    metrics?: {
      scalp?: StrategyMetricSlice;
      arb?: StrategyMetricSlice;
      combined_net_pnl_usd?: number;
      combined_trades_per_day?: number;
    };
  };
  strategy_rankings?: Array<{
    id: string;
    bot_store_keys: string[];
    win_probability: number;
    pass_probability: number;
    expected_usd_per_trade: number;
    net_pnl_usd: number;
    sharpe_approx: number;
    trades_per_day: number;
    max_drawdown_usd: number;
    confidence_score: number;
  }>;
}

export interface SimulatedBacktestResult {
  winRate: number;
  passProbability: number;
  expectedUsdPerTrade: number;
  netPnlUsd: number;
  sharpeApprox: number;
  tradesPerDay: number;
  maxDrawdownUsd: number;
  confidenceScore: number;
  durationHours: number;
  enabledEngines: string[];
  notes: string[];
  usedHoldoutData: boolean;
}

function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

function metricSlice(
  report: BacktestReportSlice | null,
  key: 'scalp' | 'arb',
): StrategyMetricSlice | null {
  const holdout = report?.retest?.metrics?.[key];
  if (holdout && holdout.total_trades > 0) return holdout;
  const baseline = report?.baseline?.metrics?.[key];
  if (baseline && baseline.total_trades > 0) return baseline;
  return null;
}

/** Heuristic slice for engines not in Rust holdout replay. */
function syntheticSlice(
  engine: 'whale_copy' | 'momentum' | 'sniper',
  config: BotStrategyConfig,
  hours: number,
): StrategyMetricSlice {
  const durationScale = hours / 6;
  const confFactor = clamp((0.95 - config.min_confidence) / 0.35, 0.4, 1.4);

  if (engine === 'whale_copy') {
    const whaleFactor = clamp(80 / config.min_whale_sol, 0.5, 2.5);
    const trades = Math.round(18 * whaleFactor * confFactor * durationScale);
    const winRate = clamp(0.58 + confFactor * 0.08, 0.45, 0.78);
    const avg = 2.2 + confFactor * 0.6;
    return {
      strategy: 'whale_copy',
      total_trades: trades,
      winning_trades: Math.round(trades * winRate),
      losing_trades: Math.round(trades * (1 - winRate)),
      net_pnl_usd: trades * avg * (winRate - (1 - winRate) * 0.6),
      win_rate: winRate,
      avg_net_per_trade_usd: avg,
      trades_per_day: (trades / hours) * 24,
      max_drawdown_usd: trades * 0.35,
      sharpe_approx: 1.2 + confFactor * 0.5,
    };
  }

  if (engine === 'momentum') {
    const trades = Math.round(120 * confFactor * durationScale);
    const winRate = clamp(0.52 + confFactor * 0.06, 0.42, 0.7);
    const avg = 0.45 + confFactor * 0.15;
    return {
      strategy: 'momentum',
      total_trades: trades,
      winning_trades: Math.round(trades * winRate),
      losing_trades: Math.round(trades * (1 - winRate)),
      net_pnl_usd: trades * avg * (winRate - (1 - winRate) * 0.55),
      win_rate: winRate,
      avg_net_per_trade_usd: avg,
      trades_per_day: (trades / hours) * 24,
      max_drawdown_usd: trades * 0.12,
      sharpe_approx: 0.9 + confFactor * 0.4,
    };
  }

  const trades = Math.round(8 * confFactor * durationScale);
  const winRate = clamp(0.48 + confFactor * 0.1, 0.38, 0.72);
  const avg = 4.5 + confFactor * 1.2;
  return {
    strategy: 'sniper',
    total_trades: trades,
    winning_trades: Math.round(trades * winRate),
    losing_trades: Math.round(trades * (1 - winRate)),
    net_pnl_usd: trades * avg * (winRate - (1 - winRate) * 0.7),
    win_rate: winRate,
    avg_net_per_trade_usd: avg,
    trades_per_day: (trades / hours) * 24,
    max_drawdown_usd: trades * 0.8,
    sharpe_approx: 1.5 + confFactor * 0.3,
  };
}

function paramMultiplier(config: BotStrategyConfig, engine: 'scalp' | 'arb'): number {
  const base = DEFAULT_STRATEGY_CONFIG;
  if (engine === 'scalp') {
    const tp = config.scalp_take_profit_pct / base.scalp_take_profit_pct;
    const sl = base.scalp_stop_loss_pct / config.scalp_stop_loss_pct;
    const conf = clamp((0.95 - config.min_confidence) / (0.95 - base.min_confidence), 0.6, 1.3);
    return clamp(tp * 0.55 + sl * 0.25 + conf * 0.2, 0.55, 1.45);
  }
  const profit = base.arb_min_profit_usd / config.arb_min_profit_usd;
  const loss = config.arb_max_loss_usd / base.arb_max_loss_usd;
  return clamp(profit * 0.7 + loss * 0.3, 0.5, 1.35);
}

function adjustSlice(slice: StrategyMetricSlice, multiplier: number): StrategyMetricSlice {
  const trades = Math.max(1, Math.round(slice.total_trades * clamp(1 / multiplier, 0.45, 1.6)));
  const winRate = clamp(slice.win_rate * clamp(multiplier, 0.85, 1.1), 0.35, 0.95);
  const avg = (slice.avg_net_per_trade_usd ?? slice.net_pnl_usd / Math.max(1, slice.total_trades)) * multiplier;
  const net = trades * avg * (winRate - (1 - winRate) * 0.5);
  return {
    ...slice,
    total_trades: trades,
    winning_trades: Math.round(trades * winRate),
    losing_trades: Math.round(trades * (1 - winRate)),
    net_pnl_usd: net,
    win_rate: winRate,
    avg_net_per_trade_usd: avg,
    trades_per_day: slice.trades_per_day
      ? slice.trades_per_day * clamp(1 / multiplier, 0.5, 1.5)
      : undefined,
    max_drawdown_usd: (slice.max_drawdown_usd ?? 0) * clamp(multiplier, 0.7, 1.4),
    sharpe_approx: (slice.sharpe_approx ?? 1) * clamp(multiplier, 0.8, 1.15),
  };
}

function rankingMatch(
  report: BacktestReportSlice | null,
  config: BotStrategyConfig,
): NonNullable<BacktestReportSlice['strategy_rankings']>[number] | null {
  const rankings = report?.strategy_rankings;
  if (!rankings?.length) return null;

  const enabled = new Set<string>();
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
    const overlap = row.bot_store_keys.filter((k) => enabled.has(k)).length;
    const score = overlap - Math.abs(row.bot_store_keys.length - enabled.size) * 0.25;
    if (score > bestScore) {
      bestScore = score;
      best = row;
    }
  }
  return best ?? null;
}

/**
 * Estimate holdout-style metrics for a strategy config.
 * Uses on-disk pipeline report when available; supplements with heuristics.
 */
export function simulateStrategyConfig(
  config: BotStrategyConfig,
  report: BacktestReportSlice | null,
  hours = 6,
): SimulatedBacktestResult {
  const notes: string[] = [];
  const enabledEngines: string[] = [];
  const slices: StrategyMetricSlice[] = [];

  if (config.scalp) {
    enabledEngines.push('scalp');
    const raw = metricSlice(report, 'scalp');
    if (raw) {
      slices.push(adjustSlice(raw, paramMultiplier(config, 'scalp')));
      notes.push('Scalp: holdout replay metrics adjusted for TP/SL.');
    } else {
      const fallback: StrategyMetricSlice = {
        strategy: 'scalp',
        total_trades: Math.round(90 * (hours / 6)),
        winning_trades: 52,
        losing_trades: 38,
        net_pnl_usd: 42,
        win_rate: 0.58,
        avg_net_per_trade_usd: 0.47,
        trades_per_day: 360,
        max_drawdown_usd: 8,
        sharpe_approx: 1.4,
      };
      slices.push(adjustSlice(fallback, paramMultiplier(config, 'scalp')));
      notes.push('Scalp: synthetic estimate (no holdout file).');
    }
  }

  if (config.arb) {
    enabledEngines.push('arb');
    const raw = metricSlice(report, 'arb');
    if (raw) {
      slices.push(adjustSlice(raw, paramMultiplier(config, 'arb')));
      notes.push('Arb: holdout replay metrics adjusted for profit gate.');
    } else {
      const fallback: StrategyMetricSlice = {
        strategy: 'arb',
        total_trades: Math.round(40 * hours),
        winning_trades: 28,
        losing_trades: 12,
        net_pnl_usd: 85,
        win_rate: 0.7,
        avg_net_per_trade_usd: 2.1,
        trades_per_day: 160,
        max_drawdown_usd: 12,
        sharpe_approx: 2.4,
      };
      slices.push(adjustSlice(fallback, paramMultiplier(config, 'arb')));
      notes.push('Arb: synthetic estimate (no holdout file).');
    }
  }

  if (config.whale_copy) {
    enabledEngines.push('whale_copy');
    slices.push(syntheticSlice('whale_copy', config, hours));
    notes.push('Whale copy: heuristic model (not in Rust holdout).');
  }
  if (config.momentum) {
    enabledEngines.push('momentum');
    slices.push(syntheticSlice('momentum', config, hours));
    notes.push('Momentum: heuristic model (not in Rust holdout).');
  }
  if (config.sniper) {
    enabledEngines.push('sniper');
    slices.push(syntheticSlice('sniper', config, hours));
    notes.push('Sniper: heuristic model (not in Rust holdout).');
  }

  if (slices.length === 0) {
    return {
      winRate: 0,
      passProbability: 0,
      expectedUsdPerTrade: 0,
      netPnlUsd: 0,
      sharpeApprox: 0,
      tradesPerDay: 0,
      maxDrawdownUsd: 0,
      confidenceScore: 0,
      durationHours: hours,
      enabledEngines: [],
      notes: ['Enable at least one strategy engine to simulate.'],
      usedHoldoutData: false,
    };
  }

  const totalTrades = slices.reduce((s, m) => s + m.total_trades, 0);
  const totalWins = slices.reduce((s, m) => s + m.winning_trades, 0);
  const netPnl = slices.reduce((s, m) => s + m.net_pnl_usd, 0);
  const winRate = totalTrades > 0 ? totalWins / totalTrades : 0;
  const expectedUsdPerTrade = totalTrades > 0 ? netPnl / totalTrades : 0;
  const tradesPerDay = slices.reduce((s, m) => s + (m.trades_per_day ?? 0), 0);
  const maxDrawdownUsd = slices.reduce((s, m) => s + (m.max_drawdown_usd ?? 0), 0);
  const sharpeApprox =
    slices.reduce((s, m) => s + (m.sharpe_approx ?? 0), 0) / slices.length;

  const ranking = rankingMatch(report, config);
  const usedHoldoutData = !!(metricSlice(report, 'scalp') || metricSlice(report, 'arb'));

  let passProbability = clamp(
    0.35 + winRate * 0.35 + (netPnl > 0 ? 0.2 : 0) + (expectedUsdPerTrade > 0.08 ? 0.1 : 0),
    0.1,
    0.98,
  );
  let confidenceScore = clamp(
    winRate * 35 + passProbability * 30 + clamp(sharpeApprox / 10, 0, 1) * 20 + clamp(expectedUsdPerTrade / 0.5, 0, 1) * 15,
    0,
    100,
  );

  if (ranking) {
    passProbability = ranking.pass_probability * 0.6 + passProbability * 0.4;
    confidenceScore = ranking.confidence_score * 0.55 + confidenceScore * 0.45;
    notes.push(`Blended with ranking match: ${ranking.id}.`);
  }

  if (!config.scalp && !config.arb && enabledEngines.length > 0) {
    notes.push('Simulation uses heuristics only — run CLI backtest for scalp/arb holdout.');
  }

  return {
    winRate,
    passProbability,
    expectedUsdPerTrade,
    netPnlUsd: netPnl,
    sharpeApprox,
    tradesPerDay,
    maxDrawdownUsd,
    confidenceScore,
    durationHours: hours,
    enabledEngines,
    notes,
    usedHoldoutData,
  };
}
