import type { QuotePairSnapshot } from '../jupiter/types.js';
import { atomicToUi } from '../jupiter/client.js';
import { estimateFeesUsd, scoreRoundTripUsd } from '../strategy/scorer.js';
import type { MarketState } from '../market/state.js';

export interface BacktestSimConfig {
  quoteStalenessMs: number;
  latencyDriftMs: number;
  landingSlippageBps: number;
  missedFillRate: number;
  priorityFeeLamports: number;
  routeChangePenaltyBps: number;
}

export const DEFAULT_BACKTEST_SIM: BacktestSimConfig = {
  quoteStalenessMs: 300,
  latencyDriftMs: 150,
  landingSlippageBps: 15,
  missedFillRate: 0.12,
  priorityFeeLamports: 80_000,
  routeChangePenaltyBps: 8,
};

export interface BacktestQuoteRecord {
  pair: QuotePairSnapshot;
  state: MarketState;
  inputDecimals: number;
}

export interface BacktestResult {
  opportunities: number;
  simulatedFills: number;
  missedFills: number;
  totalExpectedUsd: number;
  totalSimulatedUsd: number;
  avgEdgeBps: number;
}

/**
 * Replay quote pairs with execution economics — not just raw spread counting.
 */
export function runBacktest(
  records: BacktestQuoteRecord[],
  sim: BacktestSimConfig = DEFAULT_BACKTEST_SIM,
): BacktestResult {
  let opportunities = 0;
  let simulatedFills = 0;
  let missedFills = 0;
  let totalExpectedUsd = 0;
  let totalSimulatedUsd = 0;
  let edgeSum = 0;

  for (const rec of records) {
    const fees = estimateFeesUsd({
      solPriceUsd: rec.state.solPriceUsd,
      priorityFeeLamports: sim.priorityFeeLamports,
      slippageBps: sim.landingSlippageBps,
      notionalUsd:
        atomicToUi(String(rec.pair.forward.request.amount), rec.inputDecimals) *
        (rec.state.pricesUsd[rec.pair.forward.request.inputMint] ?? 0),
    });

    const scored = scoreRoundTripUsd({
      pair: rec.pair,
      state: rec.state,
      inputDecimals: rec.inputDecimals,
      outputDecimals: rec.state.decimals[rec.pair.forward.request.outputMint] ?? 6,
      feesUsd: fees,
    });

    if (scored.netProfitUsd <= 0) continue;
    opportunities += 1;
    totalExpectedUsd += scored.netProfitUsd;
    edgeSum += scored.grossSpreadBps;

    const stalePenalty =
      (sim.quoteStalenessMs + sim.latencyDriftMs) / 1000 * 0.001 * scored.startUsd;
    const routePenalty = scored.startUsd * (sim.routeChangePenaltyBps / 10_000);
    const slipPenalty = scored.startUsd * (sim.landingSlippageBps / 10_000);

    const simulated =
      scored.netProfitUsd - stalePenalty - routePenalty - slipPenalty;

    if (Math.random() < sim.missedFillRate) {
      missedFills += 1;
      continue;
    }

    simulatedFills += 1;
    totalSimulatedUsd += simulated;
  }

  return {
    opportunities,
    simulatedFills,
    missedFills,
    totalExpectedUsd,
    totalSimulatedUsd,
    avgEdgeBps: opportunities > 0 ? edgeSum / opportunities : 0,
  };
}
