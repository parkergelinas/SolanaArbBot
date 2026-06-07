import { DEFAULT_SCAN_PAIRS } from '../strategy/arb-scanner.js';
import { scanMultipleRouteDivergences } from '../strategy/multi-pair-scanner.js';
import { simulatePaperDivergence } from '../strategy/paper-quote-simulator.js';
import { USDC_MINT } from '../config/env.js';
import { evaluateQuoteFreshness, scoreRouteQuality } from '../strategy/route-quality.js';
import { estimateFeesUsd, scoreRoundTripUsd } from '../strategy/scorer.js';
export const DEFAULT_ROUTE_DIVERGENCE_CONFIG = {
    minDivergenceBps: Number(process.env.BOT_MIN_DIVERGENCE_BPS ?? '8'),
    minSurvivingEdgeBps: Number(process.env.BOT_MIN_SURVIVING_EDGE_BPS ?? '4'),
    pairsPerScan: Number(process.env.BOT_PAIRS_PER_SCAN ?? '10'),
    scanConcurrency: 3,
};
function constructionToPair(pairLabel, c) {
    return {
        pairLabel,
        forward: c.forward,
        reverse: c.reverse,
    };
}
/**
 * Route divergence arb — compare restricted vs unrestricted route constructions
 * across CMC top-100 Solana pairs (rotating batch per scan).
 */
export class RouteDivergenceArbStrategy {
    client;
    tradeAmountUi;
    pairRegistry;
    cfg;
    paperMode;
    liveQuotes;
    id = 'route_divergence_arb';
    scanTick = 0;
    constructor(client, tradeAmountUi, pairRegistry = null, cfg = DEFAULT_ROUTE_DIVERGENCE_CONFIG, paperMode = false, 
    /**
     * liveQuotes=true: use real Jupiter quote API even in paper mode.
     * Measures actual route divergence on mainnet before committing capital.
     * Execution remains simulated when paperMode=true.
     */
    liveQuotes = false) {
        this.client = client;
        this.tradeAmountUi = tradeAmountUi;
        this.pairRegistry = pairRegistry;
        this.cfg = cfg;
        this.paperMode = paperMode;
        this.liveQuotes = liveQuotes;
    }
    async scan(state) {
        this.scanTick += 1;
        const pairs = this.pairRegistry
            ? this.pairRegistry.nextBatch(this.cfg.pairsPerScan)
            : DEFAULT_SCAN_PAIRS;
        // ── Quote source selection ────────────────────────────────────────────
        //
        // Three modes:
        //   live mode  (paperMode=false):               real Jupiter quotes + real execution
        //   live-quotes (paperMode=true, liveQuotes=true): real Jupiter quotes + paper execution
        //   paper mode (paperMode=true, liveQuotes=false): stochastic simulator (no Jupiter)
        //
        // liveQuotes mode is the mainnet validation step — it measures actual route
        // divergence bps without risking capital, so we know real P&L before going live.
        const divergences = new Map();
        if (!this.paperMode || this.liveQuotes) {
            // Real Jupiter quotes (live mode OR live-quotes validation mode)
            try {
                const result = await scanMultipleRouteDivergences(this.client, pairs, this.tradeAmountUi, { slippageBps: 50, concurrency: this.cfg.scanConcurrency });
                for (const [label, div] of result.divergences) {
                    divergences.set(label, div);
                }
            }
            catch (err) {
                const msg = err instanceof Error ? err.message : String(err);
                // 429 or rate-limit: fall back to simulator in paper mode, propagate in live
                if (this.paperMode && (msg.includes('429') || msg.includes('rate-limit'))) {
                    const quotePriceUsd = state.pricesUsd[USDC_MINT] ?? 1.0;
                    for (const pair of pairs) {
                        const basePrice = state.pricesUsd[pair.baseMint];
                        if (!basePrice)
                            continue;
                        const sim = simulatePaperDivergence(pair, this.tradeAmountUi, basePrice, quotePriceUsd, this.scanTick);
                        if (sim)
                            divergences.set(pair.label, sim);
                    }
                }
                else {
                    throw err;
                }
            }
        }
        else {
            // Pure paper mode: stochastic simulator, no Jupiter calls
            const quotePriceUsd = state.pricesUsd[USDC_MINT] ?? 1.0;
            for (const pair of pairs) {
                const basePrice = state.pricesUsd[pair.baseMint];
                if (!basePrice)
                    continue;
                const sim = simulatePaperDivergence(pair, this.tradeAmountUi, basePrice, quotePriceUsd, this.scanTick);
                if (sim)
                    divergences.set(pair.label, sim);
            }
        }
        const multiDivergences = {};
        for (const [label, div] of divergences) {
            multiDivergences[label] = div;
        }
        const firstDiv = divergences.values().next().value;
        return {
            ...state,
            routeDivergence: firstDiv,
            multiDivergences,
        };
    }
    evaluate(state, ctx) {
        const allDivs = state.multiDivergences ?? {};
        const entries = Object.entries(allDivs);
        if (entries.length === 0 && state.routeDivergence) {
            return this.evaluateOne(state.routeDivergence, state, ctx, DEFAULT_SCAN_PAIRS[0]);
        }
        let best = null;
        let bestProfit = -Infinity;
        for (const [label, div] of entries) {
            const pair = this.pairRegistry?.allPairs.find((p) => p.label === label)
                ?? DEFAULT_SCAN_PAIRS.find((p) => p.label === label)
                ?? DEFAULT_SCAN_PAIRS[0];
            const decision = this.evaluateOne(div, state, ctx, pair);
            if (decision && (decision.netProfitUsd > bestProfit)) {
                bestProfit = decision.netProfitUsd;
                best = decision;
            }
        }
        return best;
    }
    evaluateOne(div, state, ctx, pair) {
        if (!div || div.constructions.length === 0)
            return null;
        const inputMint = pair.baseMint;
        const outputMint = pair.quoteMint;
        const inputDecimals = state.decimals[inputMint] ?? pair.baseDecimals;
        let bestDecision = null;
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
            // ── Quality checks: real in live/live-quotes mode, bypassed in pure paper ──
            //
            // Pure paper mode (paperMode=true, liveQuotes=false):
            //   • Quote timestamps are synthetic (always "now") — freshness check is noise
            //   • state.quality is populated from Jupiter tokens API which is rate-limited;
            //     priceRecencyMs exceeds 60s after first refresh, killing all token_quality checks
            //   → Use safe defaults (always passes)
            //
            // Live-quotes / live mode (liveQuotes=true OR paperMode=false):
            //   • Quotes are real — freshness timestamps are meaningful
            //   • state.quality comes from real Jupiter token metadata
            //   → Run real checks
            const usePaperDefaults = this.paperMode && !this.liveQuotes;
            const freshness = usePaperDefaults
                ? { score: 1, stale: false, reasons: [] }
                : evaluateQuoteFreshness(construction.forward.capturedAtMs, construction.reverse.capturedAtMs, construction.forward.response, construction.reverse.response, state.timestampMs);
            const routeQ = usePaperDefaults
                ? { score: 1, hops: 2, priceImpactPct: 0.1, tokenQualityOk: true, details: [] }
                : scoreRouteQuality({
                    forward: construction.forward.response,
                    reverse: construction.reverse.response,
                    state,
                    inputMint,
                    outputMint,
                    tradeSizeUsd: scored.startUsd,
                });
            const latencyBufferUsd = scored.startUsd * ((freshness.reasons.length * 3) / 10_000);
            const survivingEdgeBps = scored.grossSpreadBps -
                Math.round((feesUsd.totalUsd / Math.max(scored.startUsd, 1e-9)) * 10_000) -
                Math.round((latencyBufferUsd / Math.max(scored.startUsd, 1e-9)) * 10_000);
            const base = {
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
            }
            else if (freshness.stale && scored.grossSpreadBps < 25) {
                base.rejectionReason = `stale_quote:${freshness.reasons.join(',')}`;
            }
            else if (!routeQ.tokenQualityOk) {
                base.rejectionReason = 'token_quality';
            }
            else if (survivingEdgeBps < this.cfg.minSurvivingEdgeBps) {
                base.rejectionReason = `edge_eroded_by_latency_fees:${survivingEdgeBps}`;
            }
            else if (scored.netProfitUsd < ctx.minProfitUsd) {
                base.rejectionReason = 'below_min_profit';
            }
            if (scored.netProfitUsd > bestProfit) {
                bestProfit = scored.netProfitUsd;
                bestDecision = base;
            }
        }
        if (!bestDecision)
            return null;
        if (bestDecision.metadata?.construction !== div.bestConstructionLabel &&
            !bestDecision.rejectionReason) {
            bestDecision.rejectionReason = 'non_best_construction';
        }
        return bestDecision;
    }
}
