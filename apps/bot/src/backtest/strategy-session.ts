/**
 * Single-strategy paper backtest session.
 *
 * Runs a ScannableStrategy (scan + evaluate loop) for a fixed duration,
 * collecting every TradeDecision and computing performance metrics.
 *
 * The session requires a live JupiterClient because we validate real
 * quotes — this is the "live-quotes paper backtest" approach used by
 * the existing bot.  Set BOT_PAPER_MODE=1 + BOT_LIVE_QUOTES=1.
 */

import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { ScannableStrategy } from '../signals/types.js';
import type { StrategyContext, TradeDecision } from '../strategy/types.js';

export interface SessionConfig {
  /** How long to run the session in milliseconds. */
  durationMs: number;
  /** Delay between scan-evaluate iterations in milliseconds. */
  scanIntervalMs: number;
  /** Minimum net profit threshold in USD to count a trade as actionable. */
  minProfitUsd: number;
  /** Slippage tolerance in bps passed to evaluate(). */
  slippageBps: number;
  /** SOL price used when building the initial market state. */
  solPriceUsd: number;
}

export const DEFAULT_SESSION_CONFIG: SessionConfig = {
  durationMs: 20 * 60 * 1000,   // 20 minutes (override via env)
  scanIntervalMs: 15_000,        // 15 s between scans
  minProfitUsd: 0.01,
  slippageBps: 50,
  solPriceUsd: 150,
};

export interface TradeRecord {
  iterationIndex: number;
  timestampMs: number;
  decision: TradeDecision;
  /** True when decision has no rejectionReason and netProfitUsd >= minProfitUsd. */
  actionable: boolean;
}

export interface SessionMetrics {
  strategyId: string;
  durationMs: number;
  iterations: number;
  totalDecisions: number;
  actionableDecisions: number;
  rejectedDecisions: number;
  /** Trades per hour (actionable only). */
  tradesPerHour: number;
  /** Win rate = actionable / (actionable + rejected-below-profit) */
  winRate: number;
  /** Average net profit per actionable trade (USD). */
  avgNetProfitUsd: number;
  /** Cumulative net P&L across all actionable trades (USD). */
  totalNetProfitUsd: number;
  /** Gross spread bps avg across actionable trades. */
  avgGrossSpreadBps: number;
  /** Best single trade net profit (USD). */
  maxNetProfitUsd: number;
  /** Worst single trade net profit (USD). */
  minNetProfitUsd: number;
  /** Average scan duration in ms. */
  avgScanMs: number;
  records: TradeRecord[];
  errors: string[];
}

function buildInitialState(cfg: SessionConfig): MarketState {
  const SOL_MINT  = 'So11111111111111111111111111111111111111112';
  const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
  return {
    timestampMs: Date.now(),
    pricesUsd: {
      [SOL_MINT]: cfg.solPriceUsd,
      [USDC_MINT]: 1.0,
      // mSOL ≈ SOL × 1.07 (7% accumulated staking)
      'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So': cfg.solPriceUsd * 1.07,
      'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn': cfg.solPriceUsd * 1.065,
      'bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1': cfg.solPriceUsd * 1.062,
      'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB': 1.0,  // USDT
      '2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo': 1.0,  // PYUSD
    },
    decimals: {
      [SOL_MINT]: 9,
      [USDC_MINT]: 6,
      'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So': 9,
      'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn': 9,
      'bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1': 9,
      'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB': 6,
      '2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo': 6,
    },
    quality: {},
    universe: [],
    solPriceUsd: cfg.solPriceUsd,
  };
}

/**
 * Run a single strategy for `cfg.durationMs` milliseconds, collecting all
 * trade decisions.  Returns full metrics for the session.
 */
export async function runStrategySession(
  strategy: ScannableStrategy,
  _client: JupiterClient,
  cfg: SessionConfig = DEFAULT_SESSION_CONFIG,
  onProgress?: (elapsed: number, metrics: Pick<SessionMetrics, 'actionableDecisions' | 'totalNetProfitUsd' | 'tradesPerHour'>) => void,
): Promise<SessionMetrics> {
  const ctx: StrategyContext = {
    minProfitUsd: cfg.minProfitUsd,
    slippageBps: cfg.slippageBps,
  };

  let state = buildInitialState(cfg);
  const records: TradeRecord[] = [];
  const errors: string[] = [];
  const scanDurations: number[] = [];

  const startMs = Date.now();
  const endMs = startMs + cfg.durationMs;
  let iteration = 0;

  while (Date.now() < endMs) {
    const scanStart = Date.now();
    try {
      // Refresh state timestamp
      state = { ...state, timestampMs: Date.now() };

      // Scan: fetch live quotes from Jupiter
      state = await strategy.scan(state);

      // Evaluate: compute trade decision
      const decision = strategy.evaluate(state, ctx);

      if (decision) {
        const actionable = !decision.rejectionReason && decision.netProfitUsd >= cfg.minProfitUsd;
        records.push({ iterationIndex: iteration, timestampMs: Date.now(), decision, actionable });
      }

      scanDurations.push(Date.now() - scanStart);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      errors.push(`iter${iteration}: ${msg}`);
      scanDurations.push(Date.now() - scanStart);
    }

    iteration++;

    // Progress callback every 5 iterations
    if (onProgress && iteration % 5 === 0) {
      const elapsed = Date.now() - startMs;
      const actionable = records.filter((r) => r.actionable);
      const totalNet = actionable.reduce((s, r) => s + r.decision.netProfitUsd, 0);
      const tph = actionable.length / (elapsed / 3_600_000);
      onProgress(elapsed, { actionableDecisions: actionable.length, totalNetProfitUsd: totalNet, tradesPerHour: tph });
    }

    // Wait for next scan interval (account for time already spent)
    const scanElapsed = Date.now() - scanStart;
    const waitMs = Math.max(0, cfg.scanIntervalMs - scanElapsed);
    if (waitMs > 0 && Date.now() + waitMs < endMs) {
      await new Promise<void>((res) => setTimeout(res, waitMs));
    }
  }

  const actualDurationMs = Date.now() - startMs;
  const actionable = records.filter((r) => r.actionable);
  const belowProfit = records.filter((r) => r.decision.rejectionReason === 'below_min_profit');
  const totalNetUsd = actionable.reduce((s, r) => s + r.decision.netProfitUsd, 0);
  const avgNetUsd = actionable.length > 0 ? totalNetUsd / actionable.length : 0;
  const grossBpsSum = actionable.reduce((s, r) => s + r.decision.grossSpreadBps, 0);
  const avgGrossSpreadBps = actionable.length > 0 ? grossBpsSum / actionable.length : 0;
  const maxNet = actionable.reduce((m, r) => Math.max(m, r.decision.netProfitUsd), -Infinity);
  const minNet = actionable.reduce((m, r) => Math.min(m, r.decision.netProfitUsd), Infinity);
  const avgScanMs = scanDurations.reduce((s, d) => s + d, 0) / (scanDurations.length || 1);
  const tradesPerHour = actionable.length / (actualDurationMs / 3_600_000);
  const denominator = actionable.length + belowProfit.length;
  const winRate = denominator > 0 ? actionable.length / denominator : 0;

  return {
    strategyId: strategy.id,
    durationMs: actualDurationMs,
    iterations: iteration,
    totalDecisions: records.length,
    actionableDecisions: actionable.length,
    rejectedDecisions: records.filter((r) => !r.actionable).length,
    tradesPerHour,
    winRate,
    avgNetProfitUsd: avgNetUsd,
    totalNetProfitUsd: totalNetUsd,
    avgGrossSpreadBps,
    maxNetProfitUsd: Number.isFinite(maxNet) ? maxNet : 0,
    minNetProfitUsd: Number.isFinite(minNet) ? minNet : 0,
    avgScanMs,
    records,
    errors,
  };
}

/** Acceptance criteria — a strategy "passes" if all thresholds are met. */
export interface AcceptanceCriteria {
  minWinRate: number;
  minTradesPerHour: number;
  minTotalNetProfitUsd: number;
  minAvgNetProfitUsd: number;
}

export const DEFAULT_ACCEPTANCE: AcceptanceCriteria = {
  minWinRate: 0.45,
  minTradesPerHour: 1.0,
  minTotalNetProfitUsd: 0.05,
  minAvgNetProfitUsd: 0.005,
};

export interface AcceptanceResult {
  passed: boolean;
  failReasons: string[];
  metrics: SessionMetrics;
}

export function evaluateSession(
  metrics: SessionMetrics,
  criteria: AcceptanceCriteria = DEFAULT_ACCEPTANCE,
): AcceptanceResult {
  const failReasons: string[] = [];

  if (metrics.winRate < criteria.minWinRate) {
    failReasons.push(
      `win_rate ${(metrics.winRate * 100).toFixed(1)}% < ${(criteria.minWinRate * 100).toFixed(1)}%`,
    );
  }
  if (metrics.tradesPerHour < criteria.minTradesPerHour) {
    failReasons.push(
      `trades_per_hour ${metrics.tradesPerHour.toFixed(2)} < ${criteria.minTradesPerHour}`,
    );
  }
  if (metrics.totalNetProfitUsd < criteria.minTotalNetProfitUsd) {
    failReasons.push(
      `total_net_profit $${metrics.totalNetProfitUsd.toFixed(4)} < $${criteria.minTotalNetProfitUsd}`,
    );
  }
  if (metrics.avgNetProfitUsd < criteria.minAvgNetProfitUsd) {
    failReasons.push(
      `avg_net_profit $${metrics.avgNetProfitUsd.toFixed(5)} < $${criteria.minAvgNetProfitUsd}`,
    );
  }

  return {
    passed: failReasons.length === 0,
    failReasons,
    metrics,
  };
}
