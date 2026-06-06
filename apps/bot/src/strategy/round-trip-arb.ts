import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import { scanRoundTripQuotes, DEFAULT_SCAN_PAIRS } from './arb-scanner.js';
import { evaluateQuoteFreshness, scoreRouteQuality } from './route-quality.js';
import { estimateFeesUsd, scoreRoundTripUsd } from './scorer.js';
import type { Strategy, StrategyContext, TradeDecision } from './types.js';

/** Round-trip quote arbitrage — first pluggable strategy implementation. */
export class RoundTripQuoteArbStrategy implements Strategy {
  readonly id = 'round_trip_quote_arb';

  constructor(
    private readonly client: JupiterClient,
    private readonly tradeAmountUi: number,
  ) {}

  async scan(state: MarketState): Promise<MarketState> {
    const pair = DEFAULT_SCAN_PAIRS[0];
    const quotes = await scanRoundTripQuotes(
      this.client,
      pair,
      this.tradeAmountUi,
      { slippageBps: 50 },
    );
    return { ...state, quotes };
  }

  evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    if (!state.quotes) return null;

    const pair = DEFAULT_SCAN_PAIRS[0];
    const inputMint = pair.baseMint;
    const outputMint = pair.quoteMint;
    const inputDecimals = state.decimals[inputMint] ?? pair.baseDecimals;

    const feesUsd = estimateFeesUsd({
      solPriceUsd: state.solPriceUsd || state.pricesUsd[inputMint] || 0,
      slippageBps: ctx.slippageBps,
      notionalUsd: this.tradeAmountUi * (state.pricesUsd[inputMint] ?? 0),
    });

    const scored = scoreRoundTripUsd({
      pair: state.quotes,
      state,
      inputDecimals,
      outputDecimals: state.decimals[outputMint] ?? pair.quoteDecimals,
      feesUsd,
    });

    const freshness = evaluateQuoteFreshness(
      state.quotes.forward.capturedAtMs,
      state.quotes.reverse.capturedAtMs,
      state.quotes.forward.response,
      state.quotes.reverse.response,
      state.timestampMs,
    );

    const routeQ = scoreRouteQuality({
      forward: state.quotes.forward.response,
      reverse: state.quotes.reverse.response,
      state,
      inputMint,
      outputMint,
      tradeSizeUsd: scored.startUsd,
    });

    if (freshness.stale && scored.grossSpreadBps < 25) {
      return {
        strategyId: this.id,
        pairLabel: state.quotes.pairLabel,
        inputMint,
        outputMint,
        amountInAtomic: String(state.quotes.forward.request.amount),
        expectedOutAtomic: state.quotes.reverse.response.outAmount,
        netProfitUsd: scored.netProfitUsd,
        grossSpreadBps: scored.grossSpreadBps,
        routeQualityScore: routeQ.score,
        freshnessScore: freshness.score,
        rejectionReason: `stale_quote:${freshness.reasons.join(',')}`,
      };
    }

    if (!routeQ.tokenQualityOk) {
      return {
        strategyId: this.id,
        pairLabel: state.quotes.pairLabel,
        inputMint,
        outputMint,
        amountInAtomic: String(state.quotes.forward.request.amount),
        expectedOutAtomic: state.quotes.reverse.response.outAmount,
        netProfitUsd: scored.netProfitUsd,
        grossSpreadBps: scored.grossSpreadBps,
        routeQualityScore: routeQ.score,
        freshnessScore: freshness.score,
        rejectionReason: 'token_quality',
      };
    }

    if (scored.netProfitUsd < ctx.minProfitUsd) {
      return {
        strategyId: this.id,
        pairLabel: state.quotes.pairLabel,
        inputMint,
        outputMint,
        amountInAtomic: String(state.quotes.forward.request.amount),
        expectedOutAtomic: state.quotes.reverse.response.outAmount,
        netProfitUsd: scored.netProfitUsd,
        grossSpreadBps: scored.grossSpreadBps,
        routeQualityScore: routeQ.score,
        freshnessScore: freshness.score,
        rejectionReason: 'below_min_profit',
      };
    }

    return {
      strategyId: this.id,
      pairLabel: state.quotes.pairLabel,
      inputMint,
      outputMint,
      amountInAtomic: String(state.quotes.forward.request.amount),
      expectedOutAtomic: state.quotes.reverse.response.outAmount,
      netProfitUsd: scored.netProfitUsd,
      grossSpreadBps: scored.grossSpreadBps,
      routeQualityScore: routeQ.score,
      freshnessScore: freshness.score,
    };
  }
}
