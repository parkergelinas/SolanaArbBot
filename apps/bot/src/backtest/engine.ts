/**
 * Backtest engine — replays journal rejection events to find optimal adaptive
 * strategy parameters without requiring historical trade data.
 *
 * How it works:
 *  1. Extract real spread observations from rejection events
 *     (e.g. "spread_below_threshold:12bps<100bps" → observed 12 bps)
 *  2. Compute the break-even spread threshold for each capital/size combo
 *  3. Sweep minSpreadBps × minProfitUsd configs and simulate which would execute
 *  4. Return the config that maximises estimated net profit
 */

import type { JournalEvent } from '../state/journal.js';

/** Confirmed transaction cost floor from live data ($0.065 per 2-leg trade). */
const TX_COST_USD = 0.065;

export interface SpreadObservation {
  spreadBps: number;
  pairLabel: string;
  stage: string;
  tradeSizeUi: number;
  notionalUsd: number;
  grossProfitUsd: number;
  netProfitUsd: number;
}

export interface BacktestConfig {
  minSpreadBps: number;
  minProfitUsd: number;
  tradeSizeUi: number;
  solPriceUsd: number;
}

export interface BacktestResult {
  config: BacktestConfig;
  totalObservations: number;
  triggeredTrades: number;
  triggerRate: number;
  estimatedTotalProfitUsd: number;
  avgNetProfitPerTrade: number;
  profitableTrades: number;
  losingTrades: number;
  /** Annualised % return estimate (very rough — assumes same frequency). */
  estimatedAprPct: number;
}

export interface BacktestSweepResult {
  observations: SpreadObservation[];
  configs: BacktestResult[];
  best: BacktestResult | null;
  /** Break-even spread at given trade size and sol price. */
  breakEvenBps: number;
  /** Current stage's threshold vs break-even gap. */
  thresholdGapBps: number;
  recommendations: string[];
}

/** Extract real spread values from journal rejection reasons. */
export function extractSpreadObservations(
  events: readonly JournalEvent[],
  solPriceUsd = 150,
): SpreadObservation[] {
  const obs: SpreadObservation[] = [];

  for (const e of events) {
    if (e.type !== 'rejection' && e.type !== 'decision') continue;
    if (!e.rejectionReason) continue;

    // Pattern: spread_below_threshold:Xbps<Ybps[stageN:label]
    const spreadMatch = e.rejectionReason.match(
      /spread_below_threshold:(\d+)bps<(\d+)bps(?:\[([^\]]+)\])?/,
    );
    if (!spreadMatch) continue;

    const spreadBps = parseInt(spreadMatch[1]!, 10);
    const threshold = parseInt(spreadMatch[2]!, 10);
    const stage = spreadMatch[3] ?? 'unknown';

    // Get trade size from metadata if available
    const meta = (e as unknown as { metadata?: Record<string, unknown> }).metadata ?? {};
    const tradeSizeUi = (meta['tradeSizeUi'] as number | undefined) ?? 0.5;
    const notionalUsd = tradeSizeUi * solPriceUsd;
    const grossProfitUsd = (spreadBps / 10_000) * notionalUsd;
    const netProfitUsd = grossProfitUsd - TX_COST_USD;

    obs.push({
      spreadBps,
      pairLabel: e.pairLabel ?? 'unknown',
      stage,
      tradeSizeUi,
      notionalUsd,
      grossProfitUsd,
      netProfitUsd,
    });

    // Also extract the threshold itself as an implicit data point
    // (the config that was active when this rejection happened)
    void threshold; // used in pattern extraction only
  }

  return obs;
}

/** Simulate one config against the observation set. */
function simulateConfig(
  obs: SpreadObservation[],
  cfg: BacktestConfig,
  hoursOfData: number,
): BacktestResult {
  const notionalUsd = cfg.tradeSizeUi * cfg.solPriceUsd;
  let triggered = 0;
  let totalProfit = 0;
  let profitable = 0;
  let losing = 0;

  for (const o of obs) {
    if (o.spreadBps < cfg.minSpreadBps) continue;
    const gross = (o.spreadBps / 10_000) * notionalUsd;
    const net = gross - TX_COST_USD;
    if (net < cfg.minProfitUsd) continue;

    triggered += 1;
    totalProfit += net;
    if (net > 0) profitable += 1;
    else losing += 1;
  }

  const triggerRate = obs.length > 0 ? triggered / obs.length : 0;
  const avgNet = triggered > 0 ? totalProfit / triggered : 0;
  const tradesPerHour = hoursOfData > 0 ? triggered / hoursOfData : 0;
  // Very rough APR: assume same trade rate × profit continues for a year
  const capitalUsd = cfg.tradeSizeUi * cfg.solPriceUsd;
  const yearlyProfit = avgNet * tradesPerHour * 24 * 365;
  const estimatedAprPct = capitalUsd > 0 ? (yearlyProfit / capitalUsd) * 100 : 0;

  return {
    config: cfg,
    totalObservations: obs.length,
    triggeredTrades: triggered,
    triggerRate,
    estimatedTotalProfitUsd: totalProfit,
    avgNetProfitPerTrade: avgNet,
    profitableTrades: profitable,
    losingTrades: losing,
    estimatedAprPct,
  };
}

/**
 * Full parameter sweep.
 *
 * @param events       In-memory journal events
 * @param tradeSizeUi  SOL trade size (default 0.5 = Stage 1)
 * @param solPriceUsd  Current SOL price
 * @param hoursOfData  How many hours of observation data (used for rate estimates)
 */
export function runBacktest(
  events: readonly JournalEvent[],
  tradeSizeUi = 0.5,
  solPriceUsd = 150,
  hoursOfData = 1,
): BacktestSweepResult {
  const observations = extractSpreadObservations(events, solPriceUsd);

  // Break-even spread: the minimum spread needed so gross > tx cost
  const notionalUsd = tradeSizeUi * solPriceUsd;
  const breakEvenBps = Math.ceil((TX_COST_USD / notionalUsd) * 10_000);

  // Current active threshold from last rejection reason (last stage1 observation)
  const lastStage1 = [...observations].reverse().find((o) => o.stage.startsWith('stage1'));
  const currentThreshold = lastStage1
    ? observations
        .find((o) => o.pairLabel === lastStage1.pairLabel)
        ?.stage.includes('stage1')
      ? 100
      : 35
    : 100;
  const thresholdGapBps = currentThreshold - breakEvenBps;

  // Sweep grid
  const minSpreadRange = [5, 8, 10, 12, 15, 20, 25, 30, 35, 50, 75, 100, 150, 200];
  const minProfitRange = [0.005, 0.01, 0.02, 0.03, 0.05, 0.08, 0.10, 0.15, 0.20];

  const configs: BacktestResult[] = [];
  for (const minSpreadBps of minSpreadRange) {
    for (const minProfitUsd of minProfitRange) {
      configs.push(
        simulateConfig(
          observations,
          { minSpreadBps, minProfitUsd, tradeSizeUi, solPriceUsd },
          hoursOfData,
        ),
      );
    }
  }

  // Sort by estimated total profit (only count profitable trades)
  configs.sort((a, b) => b.estimatedTotalProfitUsd - a.estimatedTotalProfitUsd);
  const best = configs[0] ?? null;

  // Build recommendations
  const recommendations: string[] = [];

  if (observations.length === 0) {
    recommendations.push(
      'No spread observations in journal yet. Let the bot run for at least 30 minutes to collect data.',
    );
  } else {
    const maxSpread = Math.max(...observations.map((o) => o.spreadBps));
    const medianSpread = observations.sort((a, b) => a.spreadBps - b.spreadBps)[
      Math.floor(observations.length / 2)
    ]?.spreadBps ?? 0;
    const aboveBreakEven = observations.filter((o) => o.spreadBps >= breakEvenBps).length;
    const pctAbove = ((aboveBreakEven / observations.length) * 100).toFixed(0);

    recommendations.push(
      `Break-even spread at ${tradeSizeUi} SOL / $${notionalUsd.toFixed(0)} notional: ${breakEvenBps} bps.`,
    );
    recommendations.push(
      `Observed max spread: ${maxSpread} bps, median: ${medianSpread} bps.`,
    );
    recommendations.push(
      `${pctAbove}% of observations (${aboveBreakEven}/${observations.length}) are above break-even.`,
    );

    if (maxSpread < breakEvenBps) {
      recommendations.push(
        `⚠ ALL observed spreads are below break-even. Core pairs alone cannot cover tx costs at ${tradeSizeUi} SOL. ` +
        `Need pump pairs with ≥${breakEvenBps} bps, OR increase trade size to ≥${Math.ceil(TX_COST_USD / (maxSpread / 10_000) / solPriceUsd * 100) / 100} SOL.`,
      );
    } else if (best && best.triggeredTrades > 0) {
      recommendations.push(
        `Optimal config: minSpreadBps=${best.config.minSpreadBps}, minProfitUsd=$${best.config.minProfitUsd} ` +
        `→ ${best.triggeredTrades} trades, $${best.estimatedTotalProfitUsd.toFixed(4)} total profit.`,
      );
      if (best.config.minSpreadBps < currentThreshold) {
        recommendations.push(
          `Current threshold (${currentThreshold} bps) is ${thresholdGapBps} bps above optimal. ` +
          `Lowering to ${best.config.minSpreadBps} bps could unlock ${best.triggeredTrades} additional trades.`,
        );
      }
    }

    if (aboveBreakEven === 0 && maxSpread > 0) {
      recommendations.push(
        `Strategy is working correctly — it's correctly filtering unprofitable spreads. ` +
        `Core pair spreads (${maxSpread} bps max) are below the break-even line for this capital level. ` +
        `Solution: grow to Stage 3 (10+ SOL) where break-even drops to ~2 bps, or wait for pump pair discovery.`,
      );
    }
  }

  return {
    observations,
    configs: configs.slice(0, 20), // top 20 configs
    best,
    breakEvenBps,
    thresholdGapBps,
    recommendations,
  };
}
