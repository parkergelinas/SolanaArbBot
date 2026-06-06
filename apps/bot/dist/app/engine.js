import { loadEnv, SOL_MINT, USDC_MINT } from '../config/env.js';
import { createMarketStack } from '../market-data/index.js';
import { computeDynamicSize } from '../sizing/dynamic.js';
import { checkStrategyRisk, DEFAULT_RISK_LIMITS, recordExecutionOutcome, } from '../risk/index.js';
import { DEFAULT_MEAN_REVERSION_CONFIG, MeanReversionStrategy, RouteDivergenceArbStrategy, RoundTripQuoteArbStrategy, StrategyRegistry, } from '../signals/index.js';
import { TradeJournal } from '../state/journal.js';
import { computeAnalytics } from '../analytics/metrics.js';
import { LiveExecutor } from '../execution/index.js';
/**

 * Main orchestrator — 4-layer pipeline:

 * MarketData → Signal → Risk → Execution

 */
export class BotEngine {
    env = loadEnv();
    journal = new TradeJournal();
    registry = new StrategyRegistry();
    stack = createMarketStack(this.env);
    executor;
    scannable = [];
    riskState = {
        sessionLossUsd: 0,
        consecutiveQuoteFailures: 0,
        inventoryUsd: {},
        routeFamilyFailures: {},
        cooldownUntilMs: 0,
    };
    stats = { scans: 0, actionable: 0, executed: 0, rejected: 0 };
    failureRate = 0;
    running = false;
    tradeAmountUi;
    constructor() {
        this.tradeAmountUi = this.env.tradeAmountUi;
        const routeDiv = new RouteDivergenceArbStrategy(this.stack.client, this.tradeAmountUi);
        const roundTrip = new RoundTripQuoteArbStrategy(this.stack.client, this.tradeAmountUi);
        const meanRev = new MeanReversionStrategy({
            ...DEFAULT_MEAN_REVERSION_CONFIG,
            enabled: this.env.enableMeanReversion,
        });
        this.scannable.push(routeDiv, roundTrip);
        this.registry.register(routeDiv);
        this.registry.register(roundTrip);
        this.registry.register(meanRev);
        this.executor = new LiveExecutor(this.env, this.stack.client, this.journal);
    }
    getStats() {
        return { ...this.stats };
    }
    getJournal() {
        return this.journal;
    }
    getAnalytics() {
        return computeAnalytics(this.journal.all());
    }
    getStrategies() {
        return this.registry.list();
    }
    async scanOnce() {
        this.stats.scans += 1;
        this.journal.append({ type: 'scan', pairLabel: 'SOL/USDC' });
        let state = await this.stack.dataLayer.buildState([SOL_MINT, USDC_MINT], {
            alwaysInclude: [SOL_MINT, USDC_MINT],
            tokenLimit: 100,
        });
        for (const strat of this.scannable) {
            state = await strat.scan(state);
        }
        const decision = this.pickDecision(state);
        if (!decision)
            return;
        this.journal.append({
            type: decision.rejectionReason ? 'rejection' : 'decision',
            strategyId: decision.strategyId,
            pairLabel: decision.pairLabel,
            expectedProfitUsd: decision.netProfitUsd,
            rejectionReason: decision.rejectionReason,
            routeMetadata: {
                grossSpreadBps: decision.grossSpreadBps,
                routeQuality: decision.routeQualityScore,
                freshness: decision.freshnessScore,
                ...decision.metadata,
            },
        });
        const risk = checkStrategyRisk(decision, this.riskState, DEFAULT_RISK_LIMITS);
        if (risk.verdict !== 'allow') {
            this.stats.rejected += 1;
            if (risk.verdict === 'halt') {
                this.journal.append({ type: 'halt', rejectionReason: risk.reason });
                this.stop();
            }
            return;
        }
        this.stats.actionable += 1;
        const liq = state.quality[decision.inputMint]?.liquidityUsd ?? 0;
        this.tradeAmountUi = computeDynamicSize({
            baseAmountUi: this.env.tradeAmountUi,
            spreadBps: decision.grossSpreadBps,
            liquidityUsd: liq,
            volatilityPct: 2,
            routeQualityScore: decision.routeQualityScore,
            recentFailureRate: this.failureRate,
            minAmountUi: 0.1,
            maxAmountUi: 10,
        });
        const result = await this.executor.execute(decision);
        this.riskState = recordExecutionOutcome(this.riskState, decision, result.realizedProfitUsd ?? 0, result.success);
        if (result.success) {
            this.stats.executed += 1;
        }
        else {
            this.stats.rejected += 1;
            this.failureRate = Math.min(1, this.failureRate + 0.05);
        }
    }
    pickDecision(state) {
        const ctx = {
            minProfitUsd: this.env.minProfitUsd,
            slippageBps: this.env.slippageBps,
        };
        if (this.env.primaryStrategy === 'round_trip_quote_arb') {
            const rt = this.registry
                .evaluateAll(state, ctx)
                .find((d) => d.strategyId === 'round_trip_quote_arb');
            return rt ?? this.registry.best(state, ctx);
        }
        const routeDiv = this.registry
            .evaluateAll(state, ctx)
            .find((d) => d.strategyId === 'route_divergence_arb');
        return routeDiv ?? this.registry.best(state, ctx);
    }
    async runLoop(maxIterations) {
        this.running = true;
        let i = 0;
        while (this.running) {
            await this.scanOnce();
            i += 1;
            if (maxIterations && i >= maxIterations)
                break;
            await new Promise((r) => setTimeout(r, this.env.scanIntervalMs));
        }
    }
    stop() {
        this.running = false;
    }
}
