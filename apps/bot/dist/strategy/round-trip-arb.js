import { scanMultipleRoundTrips } from './multi-pair-scanner.js';
import { DEFAULT_SCAN_PAIRS } from './arb-scanner.js';
import { evaluateQuoteFreshness, scoreRouteQuality } from './route-quality.js';
import { estimateFeesUsd, scoreRoundTripUsd } from './scorer.js';
export const DEFAULT_ROUND_TRIP_CONFIG = {
    pairsPerScan: 10,
    scanConcurrency: 3,
};
/** Round-trip quote arbitrage across CMC top-100 Solana pairs. */
export class RoundTripQuoteArbStrategy {
    client;
    tradeAmountUi;
    pairRegistry;
    cfg;
    id = 'round_trip_quote_arb';
    constructor(client, tradeAmountUi, pairRegistry = null, cfg = DEFAULT_ROUND_TRIP_CONFIG) {
        this.client = client;
        this.tradeAmountUi = tradeAmountUi;
        this.pairRegistry = pairRegistry;
        this.cfg = cfg;
    }
    async scan(state) {
        const pairs = this.pairRegistry
            ? this.pairRegistry.nextBatch(this.cfg.pairsPerScan)
            : DEFAULT_SCAN_PAIRS;
        const result = await scanMultipleRoundTrips(this.client, pairs, this.tradeAmountUi, { slippageBps: 50, concurrency: this.cfg.scanConcurrency });
        const multiQuotes = {};
        for (const [label, quote] of result.quotes) {
            multiQuotes[label] = quote;
        }
        const firstQuote = result.quotes.values().next().value;
        return { ...state, quotes: firstQuote, multiQuotes };
    }
    evaluate(state, ctx) {
        const allQuotes = state.multiQuotes ?? {};
        const entries = Object.entries(allQuotes);
        if (entries.length === 0 && state.quotes) {
            return this.evaluateOne(state.quotes, state, ctx, DEFAULT_SCAN_PAIRS[0]);
        }
        let best = null;
        let bestProfit = -Infinity;
        for (const [label, quotes] of entries) {
            const pair = this.pairRegistry?.allPairs.find((p) => p.label === label)
                ?? DEFAULT_SCAN_PAIRS.find((p) => p.label === label)
                ?? DEFAULT_SCAN_PAIRS[0];
            const decision = this.evaluateOne(quotes, state, ctx, pair);
            if (decision && decision.netProfitUsd > bestProfit) {
                bestProfit = decision.netProfitUsd;
                best = decision;
            }
        }
        return best;
    }
    evaluateOne(quotes, state, ctx, pair) {
        const inputMint = pair.baseMint;
        const outputMint = pair.quoteMint;
        const inputDecimals = state.decimals[inputMint] ?? pair.baseDecimals;
        const feesUsd = estimateFeesUsd({
            solPriceUsd: state.solPriceUsd || (state.pricesUsd[inputMint] ?? 0),
            slippageBps: ctx.slippageBps,
            notionalUsd: this.tradeAmountUi * (state.pricesUsd[inputMint] ?? 0),
        });
        const scored = scoreRoundTripUsd({
            pair: quotes,
            state,
            inputDecimals,
            outputDecimals: state.decimals[outputMint] ?? pair.quoteDecimals,
            feesUsd,
        });
        const freshness = evaluateQuoteFreshness(quotes.forward.capturedAtMs, quotes.reverse.capturedAtMs, quotes.forward.response, quotes.reverse.response, state.timestampMs);
        const routeQ = scoreRouteQuality({
            forward: quotes.forward.response,
            reverse: quotes.reverse.response,
            state,
            inputMint,
            outputMint,
            tradeSizeUsd: scored.startUsd,
        });
        const base = {
            strategyId: this.id,
            pairLabel: quotes.pairLabel,
            inputMint,
            outputMint,
            amountInAtomic: String(quotes.forward.request.amount),
            expectedOutAtomic: quotes.reverse.response.outAmount,
            netProfitUsd: scored.netProfitUsd,
            grossSpreadBps: scored.grossSpreadBps,
            routeQualityScore: routeQ.score,
            freshnessScore: freshness.score,
        };
        if (freshness.stale && scored.grossSpreadBps < 25) {
            return { ...base, rejectionReason: `stale_quote:${freshness.reasons.join(',')}` };
        }
        if (!routeQ.tokenQualityOk) {
            return { ...base, rejectionReason: 'token_quality' };
        }
        if (scored.netProfitUsd < ctx.minProfitUsd) {
            return { ...base, rejectionReason: 'below_min_profit' };
        }
        return base;
    }
}
