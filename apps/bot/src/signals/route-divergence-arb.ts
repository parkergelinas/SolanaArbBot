import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState, RouteConstructionSnapshot } from '../market/state.js';
import { DEFAULT_SCAN_PAIRS } from '../strategy/arb-scanner.js';
import { scanRouteDivergence } from '../strategy/route-divergence-scanner.js';
import { evaluateQuoteFreshness, scoreRouteQuality } from '../strategy/route-quality.js';
import { estimateFeesUsd, scoreRoundTripUsd } from '../strategy/scorer.js';
import type { QuotePairSnapshot } from '../jupiter/types.js';
import type { ScannableStrategy } from './types.js';
import type { StrategyContext, TradeDecision } from '../strategy/types.js';

export interface RouteDivergenceConfig {
  minDivergenceBps: number;
  minSurvivingEdgeBps: number;
}

export const DEFAULT_ROUTE_DIVERGENCE_CONFIG: RouteDivergenceConfig = {
  minDivergenceBps: 12,
  minSurvivingEdgeBps: 8,
};

function constructionToPair(
  pairLabel: string,
  c: RouteConstructionSnapshot,
): QuotePairSnapshot {
  return {
    pairLabel,
    forward: c.forward,
    reverse: c.reverse,
  };
}

/**
 * Route divergence arb — compare restricted vs unrestricted (and alternate)
 * route constructions; trade only when spread survives fees, latency, and staleness.
 */
export class RouteDivergenceArbStrategy implements ScannableStrategy {
  readonly id = 'route_divergence_arb';

  constructor(
    private readonly client: JupiterClient,
    private readonly tradeAmountUi: number,
    private readonly cfg: RouteDivergenceConfig = DEFAULT_ROUTE_DIVERGENCE_CONFIG,
  ) {}

  async scan(state: MarketState): Promise<MarketState> {
    const pair = DEFAULT_SCAN_PAIRS[0]!;
    const divergence = await scanRouteDivergence(
      this.client,
      pair,
      this.tradeAmountUi,
      { slippageBps: 50 },
    );
    return { ...state, routeDivergence: divergence };
  }

  evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    const div = state.routeDivergence;
    if (!div || div.constructions.length === 0) return null;

    const pair = DEFAULT_SCAN_PAIRS[0]!;
    const inputMint = pair.baseMint;
    const outputMint = pair.quoteMint;
    const inputDecimals = state.decimals[inputMint] ?? pair.baseDecimals;

    let bestDecision: TradeDecision | null = null;
    let bestProfit = -Infinity;

    for (const construction of div.constructions) {
      const quotePair = constructionToPair(div.pairLabel, construction);
      const feesUsd = estimateFeesUsd({
        solPriceUsd: state.solPriceUsd || state.pricesUsd[inputMint] || 0,
        slippageBps: ctx.slippageBps,
        notionalUsd: this.tradeAmountUi * (state.pricesUsd[inputMint] ?? 0),
      });

      const scored = scoreRoundTripUsd({
        pair: quotePair,
        state,
        inputDecimals,
        outputDecimals: state.decimals[outputMint] ?? pair.quoteDecimals,
        feesUsd,
      });

      const freshness = evaluateQuoteFreshness(
        construction.forward.capturedAtMs,
        construction.reverse.capturedAtMs,
        construction.forward.response,
        construction.reverse.response,
        state.timestampMs,
      );

      const routeQ = scoreRouteQuality({
        forward: construction.forward.response,
        reverse: construction.reverse.response,
        state,
        inputMint,
        outputMint,
        tradeSizeUsd: scored.startUsd,
      });

      const latencyBufferUsd =
        scored.startUsd * ((freshness.reasons.length * 3) / 10_000);
      const survivingEdgeBps =
        scored.grossSpreadBps -
        Math.round((feesUsd.totalUsd / Math.max(scored.startUsd, 1e-9)) * 10_000) -
        Math.round((latencyBufferUsd / Math.max(scored.startUsd, 1e-9)) * 10_000);

      const base: TradeDecision = {
        strategyId: this.id,
        pairLabel: `${div.pairLabel}:${construction.label}`,
        inputMint,
        outputMint,
        amountInAtomic: String(construction.forward.request.amount),
        expectedOutAtomic: construction.reverse.response.outAmount,
        netProfitUsd: scored.netProfitUsd,
        grossSpreadBps: scored.grossSpreadBps,
        routeQualityScore: routeQ.score,
        freshnessScore: freshness.score,
        metadata: {
          construction: construction.label,
          divergenceBps: div.divergenceBps,
          survivingEdgeBps,
          restrictIntermediateTokens: construction.restrictIntermediateTokens,
        },
      };

      if (div.divergenceBps < this.cfg.minDivergenceBps) {
        base.rejectionReason = `low_route_divergence:${div.divergenceBps}`;
      } else if (freshness.stale && scored.grossSpreadBps < 25) {
        base.rejectionReason = `stale_quote:${freshness.reasons.join(',')}`;
      } else if (!routeQ.tokenQualityOk) {
        base.rejectionReason = 'token_quality';
      } else if (survivingEdgeBps < this.cfg.minSurvivingEdgeBps) {
        base.rejectionReason = `edge_eroded_by_latency_fees:${survivingEdgeBps}`;
      } else if (scored.netProfitUsd < ctx.minProfitUsd) {
        base.rejectionReason = 'below_min_profit';
      }

      if (scored.netProfitUsd > bestProfit) {
        bestProfit = scored.netProfitUsd;
        bestDecision = base;
      }
    }

    if (!bestDecision) return null;

    if (
      bestDecision.metadata?.construction !== div.bestConstructionLabel &&
      !bestDecision.rejectionReason
    ) {
      bestDecision.rejectionReason = 'non_best_construction';
    }

    return bestDecision;
  }
}
