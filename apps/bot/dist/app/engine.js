import { loadEnv, SOL_MINT, USDC_MINT } from '../config/env.js';
import { createMarketStack } from '../market/data-layer.js';
import { computeDynamicSize } from '../sizing/dynamic.js';
import { checkStrategyRisk, DEFAULT_RISK_LIMITS, recordExecutionOutcome, } from '../risk/limits.js';
import { StrategyRegistry } from '../strategy/registry.js';
import { RoundTripQuoteArbStrategy } from '../strategy/round-trip-arb.js';
import { TradeJournal } from '../state/journal.js';
import { computeAnalytics } from '../analytics/metrics.js';
import { LiveExecutor } from '../execution/live-executor.js';
/** Main scan loop — market data → strategy → risk → sizing → execution. */
export class BotEngine {
    env = loadEnv();
    journal = new TradeJournal();
    registry = new StrategyRegistry();
    stack = createMarketStack(this.env);
    executor;
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
    quoteArb;
    tradeAmountUi;
    constructor() {
        this.tradeAmountUi = this.env.tradeAmountUi;
        this.quoteArb = new RoundTripQuoteArbStrategy(this.stack.client, this.tradeAmountUi);
        this.registry.register(this.quoteArb);
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
    async scanOnce() {
        this.stats.scans += 1;
        this.journal.append({ type: 'scan', pairLabel: 'SOL/USDC' });
        let state = await this.stack.dataLayer.buildState([SOL_MINT, USDC_MINT], {
            alwaysInclude: [SOL_MINT, USDC_MINT],
            tokenLimit: 100,
        });
        const strat = this.quoteArb;
        state = await strat.scan(state);
        const decision = this.registry.best(state, {
            minProfitUsd: this.env.minProfitUsd,
            slippageBps: this.env.slippageBps,
        });
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
