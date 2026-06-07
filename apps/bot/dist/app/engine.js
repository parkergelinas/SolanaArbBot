import { Connection } from '@solana/web3.js';
import { loadEnv, SOL_MINT, USDC_MINT } from '../config/env.js';
import { logger } from '../logger.js';
import { getTotalPnlUsd } from '../db/sqlite.js';
import { HardenedExecutor } from '../execution/hardened-executor.js';
import { startMonitoringServer } from '../monitoring/server.js';
import { alertHalt, checkPnlAlert } from '../monitoring/alerts.js';
import { OpportunityDetector } from '../detector/opportunity-detector.js';
import { createMarketStack } from '../market-data/index.js';
import { pairRegistryFromEnv } from '../market/pair-registry.js';
import { computeDynamicSize } from '../sizing/dynamic.js';
import { checkStrategyRisk, DEFAULT_RISK_LIMITS, recordExecutionOutcome, } from '../risk/index.js';
import { DEFAULT_MEAN_REVERSION_CONFIG, MeanReversionStrategy, PumpEdgeStrategy, RouteDivergenceArbStrategy, RoundTripQuoteArbStrategy, StrategyRegistry, } from '../signals/index.js';
import { TradeJournal } from '../state/journal.js';
// ── Well-known Orca Whirlpool pool metadata ───────────────────────────────────
// Keyed by on-chain pool address. Add more pools here as needed.
const WELL_KNOWN_ORCA_POOLS = {
    // SOL/USDC — Orca Whirlpool 0.05% fee tier (mainnet)
    'HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ': {
        dexName: 'orca',
        tokenMintA: SOL_MINT,
        tokenMintB: USDC_MINT,
        decimalsA: 9,
        decimalsB: 6,
    },
    // SOL/USDC — Orca Whirlpool 0.3% fee tier (mainnet)
    'EGZ7tiLeH62TPV1gL8WwbXGzEPa9zmcpVnnkPKKnrE2U': {
        dexName: 'orca',
        tokenMintA: SOL_MINT,
        tokenMintB: USDC_MINT,
        decimalsA: 9,
        decimalsB: 6,
    },
};
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
    pairRegistry = pairRegistryFromEnv(this.stack.client, this.env);
    executor;
    hardenedExecutor;
    opportunityDetector;
    stopMonitor = null;
    inFlightCount = 0;
    scannable = [];
    pumpStrategy = null;
    riskState = {
        sessionLossUsd: 0,
        consecutiveQuoteFailures: 0,
        inventoryUsd: {},
        routeFamilyFailures: {},
        cooldownUntilMs: 0,
    };
    /** Dedicated Connection for priority-fee sampling (separate from JupiterClient internals). */
    connection;
    stats = { scans: 0, actionable: 0, executed: 0, rejected: 0, pairCount: 0 };
    failureRate = 0;
    running = false;
    tradeAmountUi;
    initialized = false;
    /** Cached SOL price — refreshed every scan, used by event-driven arb handler. */
    lastSolPriceUsd = 170;
    /** Cached priority fee — refreshed every N scans to avoid per-scan RPC overhead. */
    lastPriorityFeeMicroLamports = 50_000;
    priorityFeeRefreshAt = 0; // epoch ms
    constructor() {
        this.connection = new Connection(this.env.rpcUrl, 'confirmed');
        this.tradeAmountUi = this.env.tradeAmountUi;
        const routeDiv = new RouteDivergenceArbStrategy(this.stack.client, this.tradeAmountUi, this.pairRegistry, {
            minDivergenceBps: Number(process.env.BOT_MIN_DIVERGENCE_BPS ?? '5'),
            minSurvivingEdgeBps: Number(process.env.BOT_MIN_SURVIVING_EDGE_BPS ?? '3'),
            pairsPerScan: this.env.pairsPerScan,
            scanConcurrency: 2,
        }, this.env.paperMode, // paper execution (no on-chain txns)
        this.env.liveQuotes);
        const roundTrip = new RoundTripQuoteArbStrategy(this.stack.client, this.tradeAmountUi, this.pairRegistry, { pairsPerScan: this.env.pairsPerScan, scanConcurrency: 3 });
        const meanRev = new MeanReversionStrategy({
            ...DEFAULT_MEAN_REVERSION_CONFIG,
            enabled: this.env.enableMeanReversion,
        });
        this.scannable.push(routeDiv, roundTrip);
        this.registry.register(routeDiv);
        this.registry.register(roundTrip);
        this.registry.register(meanRev);
        if (this.env.enablePumpEdge) {
            this.pumpStrategy = new PumpEdgeStrategy(this.stack.client, this.env.rpcUrl, undefined, this.env.heliusApiKey);
            this.scannable.push(this.pumpStrategy);
            this.registry.register(this.pumpStrategy);
        }
        this.executor = new LiveExecutor(this.env, this.stack.client, this.journal);
        this.hardenedExecutor = new HardenedExecutor(this.env, this.stack.client, this.journal, {
            jitoEnabled: this.env.jitoEnabled,
            jitoTipLamports: this.env.jitoTipLamports,
            minProfitLamports: this.env.minProfitLamports,
        });
        // ── Cross-DEX opportunity detector (Orca WebSocket + Jupiter polling) ──
        this.opportunityDetector = new OpportunityDetector(this.env.rpcUrl, {
            minProfitLamports: this.env.minProfitLamports,
            solPriceUsd: 170, // updated from live market state on every scan
            priorityFeeLamports: 50_000,
            maxPriceStaleMs: 15_000,
            swapFeeBps: 30,
        });
        // Register Orca Whirlpool subscriptions (env pools + defaults)
        const poolsToWatch = this.env.orcaPoolAddresses.length > 0
            ? this.env.orcaPoolAddresses
            : Object.keys(WELL_KNOWN_ORCA_POOLS);
        for (const addr of poolsToWatch) {
            const meta = WELL_KNOWN_ORCA_POOLS[addr];
            if (!meta) {
                logger.warn({ addr }, 'engine: Orca pool not in well-known map — skipping (add metadata to WELL_KNOWN_ORCA_POOLS)');
                continue;
            }
            this.opportunityDetector.watchOrcaPool({ poolAddress: addr, ...meta });
        }
        this.opportunityDetector.on('opportunity', (opp) => {
            this.handleExternalOpportunity(opp).catch((err) => logger.error({ err }, 'engine: handleExternalOpportunity error'));
        });
        // Propagate dead-man's-switch halt to the engine loop
        this.hardenedExecutor.deadManSwitch.on('halt', async (evt) => {
            logger.error(evt, 'engine: dead-man-switch triggered halt');
            await alertHalt(evt.reason).catch(() => undefined);
            this.stop();
        });
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
    getPairCount() {
        return this.pairRegistry.allPairs.length;
    }
    async ensureInitialized() {
        if (this.initialized)
            return;
        const pairs = await this.pairRegistry.refresh();
        this.stats.pairCount = pairs.length;
        if (this.pumpStrategy) {
            for (const meta of this.pairRegistry.pairMeta) {
                this.pumpStrategy.watchMint(meta.mint);
            }
        }
        logger.info({ pairs: pairs.length, source: this.env.pairSource, batch: this.env.pairsPerScan }, 'bot: pair universe loaded');
        this.initialized = true;
    }
    async scanOnce() {
        await this.ensureInitialized();
        this.stats.scans += 1;
        const batchLabel = `batch-${this.stats.scans}`;
        this.journal.append({ type: 'scan', pairLabel: batchLabel });
        const allMints = this.pairRegistry.allPairs.flatMap((p) => [p.baseMint, p.quoteMint]);
        const uniqueMints = [...new Set(allMints)];
        let state = await this.stack.dataLayer.buildState(uniqueMints, {
            alwaysInclude: [SOL_MINT, USDC_MINT, ...uniqueMints.slice(0, 50)],
            tokenLimit: 200,
        });
        // Cache SOL price for event-driven handler (no access to state there).
        this.lastSolPriceUsd = state.solPriceUsd;
        // Refresh priority fee estimate every 60 s (10 scans at default 6-s interval).
        // Avoids adding a synchronous RPC call on every scan while keeping the value fresh.
        const now = Date.now();
        if (now > this.priorityFeeRefreshAt) {
            this.connection
                .getRecentPrioritizationFees()
                .then((fees) => {
                if (!fees.length)
                    return;
                const sorted = fees.map((f) => f.prioritizationFee).sort((a, b) => a - b);
                const p75 = sorted[Math.floor(sorted.length * 0.75)] ?? 50_000;
                this.lastPriorityFeeMicroLamports = Math.max(5_000, Math.min(500_000, p75));
            })
                .catch(() => undefined);
            this.priorityFeeRefreshAt = now + 60_000;
        }
        // Keep the cross-DEX detector's fee math current with live SOL price
        this.opportunityDetector.updateSolPrice(state.solPriceUsd);
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
        const sizing = computeDynamicSize({
            baseAmountUi: this.env.tradeAmountUi,
            spreadBps: decision.grossSpreadBps,
            liquidityUsd: liq,
            volatilityPct: 2,
            routeQualityScore: decision.routeQualityScore,
            recentFailureRate: this.failureRate,
            minAmountUi: 0.1,
            maxAmountUi: Math.min(1.0, this.env.tradeAmountUi * 2), // hard cap: 1 SOL
            solPriceUsd: state.solPriceUsd,
            jitoActive: this.env.jitoEnabled,
            priorityFeeMicroLamports: this.lastPriorityFeeMicroLamports,
        });
        this.tradeAmountUi = sizing.amountUi;
        logger.debug({ sizing: sizing.rationale, pair: decision.pairLabel }, 'bot: trade size computed');
        // Use hardened executor in live mode, paper executor otherwise
        this.inFlightCount += 1;
        const result = this.env.paperMode
            ? await this.executor.execute(decision)
            : await this.hardenedExecutor.execute(decision);
        this.inFlightCount -= 1;
        this.riskState = recordExecutionOutcome(this.riskState, decision, result.realizedProfitUsd ?? 0, result.success);
        if (result.success) {
            this.stats.executed += 1;
            // Decay failure rate on each success (halved toward zero, floor 0)
            this.failureRate = Math.max(0, this.failureRate - 0.1);
            logger.info({ pair: decision.pairLabel, profit: result.realizedProfitUsd, sig: result.signature }, 'bot: trade executed');
            // Alert if session PnL drops below threshold
            await checkPnlAlert(getTotalPnlUsd(), state.solPriceUsd).catch(() => undefined);
        }
        else {
            this.stats.rejected += 1;
            this.failureRate = Math.min(1, this.failureRate + 0.05);
            logger.warn({ pair: decision.pairLabel, error: result.error }, 'bot: trade failed');
        }
    }
    pickDecision(state) {
        const ctx = {
            minProfitUsd: this.env.minProfitUsd,
            slippageBps: this.env.slippageBps,
        };
        if (this.env.primaryStrategy === 'pump_edge' && this.pumpStrategy) {
            const pump = this.registry
                .evaluateAll(state, ctx)
                .find((d) => d.strategyId === 'pump_edge');
            if (pump && !pump.rejectionReason)
                return pump;
        }
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
    /**
     * Execute a cross-DEX opportunity emitted by the OpportunityDetector.
     * Runs outside the scan loop (event-driven), so uses its own inFlightCount slot.
     */
    async handleExternalOpportunity(opp) {
        if (!this.running)
            return;
        if (this.hardenedExecutor.deadManSwitch.isHalted())
            return;
        // Compute a fresh size for this specific opportunity — the event fires
        // independently of the scan loop so we can't rely on this.tradeAmountUi.
        const extSizing = computeDynamicSize({
            baseAmountUi: this.env.tradeAmountUi,
            spreadBps: opp.grossSpreadBps,
            liquidityUsd: 500_000, // Orca SOL/USDC is deep; conservative floor for unknowns
            volatilityPct: 2,
            routeQualityScore: 0.8,
            recentFailureRate: this.failureRate,
            minAmountUi: 0.1,
            maxAmountUi: Math.min(1.0, this.env.tradeAmountUi * 2),
            solPriceUsd: this.lastSolPriceUsd,
            jitoActive: this.env.jitoEnabled,
            priorityFeeMicroLamports: this.lastPriorityFeeMicroLamports,
        });
        logger.debug({ sizing: extSizing.rationale, opp: `${opp.dex1}→${opp.dex2}` }, 'bot: cross-dex trade size computed');
        const decision = {
            strategyId: 'cross_dex_arb',
            pairLabel: `${opp.dex1}→${opp.dex2}:${opp.tokenIn.slice(0, 6)}`,
            inputMint: opp.tokenIn,
            outputMint: opp.tokenOut,
            amountInAtomic: String(Math.floor(extSizing.amountUi * 1e9)),
            expectedOutAtomic: '0',
            netProfitUsd: opp.profitUsd,
            grossSpreadBps: opp.grossSpreadBps,
            routeQualityScore: 0.8,
            freshnessScore: 1.0,
            metadata: {
                source: 'cross_dex_detector',
                dex1: opp.dex1,
                dex2: opp.dex2,
                profitToRiskRatio: opp.profitToRiskRatio,
            },
        };
        const risk = checkStrategyRisk(decision, this.riskState, DEFAULT_RISK_LIMITS);
        if (risk.verdict !== 'allow') {
            logger.debug({ verdict: risk.verdict, reason: risk.reason }, 'engine: cross-dex opp blocked by risk layer');
            return;
        }
        this.stats.actionable += 1;
        this.inFlightCount += 1;
        const result = this.env.paperMode
            ? await this.executor.execute(decision)
            : await this.hardenedExecutor.execute(decision);
        this.inFlightCount -= 1;
        this.riskState = recordExecutionOutcome(this.riskState, decision, result.realizedProfitUsd ?? 0, result.success);
        if (result.success) {
            this.stats.executed += 1;
            this.failureRate = Math.max(0, this.failureRate - 0.1);
            logger.info({ pair: decision.pairLabel, profit: result.realizedProfitUsd, sig: result.signature }, 'bot: cross-dex trade executed');
        }
        else {
            this.stats.rejected += 1;
            this.failureRate = Math.min(1, this.failureRate + 0.05);
            logger.warn({ pair: decision.pairLabel, error: result.error }, 'bot: cross-dex trade failed');
        }
    }
    async runLoop(maxIterations) {
        this.running = true;
        // Start Express monitoring dashboard
        this.stopMonitor = startMonitoringServer({
            port: this.env.monitorPort,
            rpcUrl: this.env.rpcUrl,
            walletPublicKey: this.env.walletPublicKey,
            isHealthy: () => this.running && !this.hardenedExecutor.deadManSwitch.isHalted(),
            journal: this.journal,
            getStats: () => this.getStats(),
        });
        // Start price polling for the cross-DEX detector.
        // In paper mode we skip the Jupiter price poll entirely to avoid hammering
        // the rate-limited public API. DexScreener (via buildState fallback) covers
        // price discovery for the scan loop — the cross-DEX detector is event-driven
        // so it doesn't need proactive price refreshes during paper sessions.
        const pairMints = [...new Set(this.pairRegistry.allPairs.map((p) => p.baseMint))];
        // Start Jupiter price polling when running live quotes or live mode.
        // Pure paper mode skips this to avoid hammering the rate-limited public API.
        if (!this.env.paperMode || this.env.liveQuotes) {
            this.opportunityDetector.startJupiterPolling(async (mints) => {
                const resp = await this.stack.client.getPrices(mints);
                const out = {};
                for (const [mint, entry] of Object.entries(resp)) {
                    if (entry.usdPrice != null && entry.usdPrice > 0)
                        out[mint] = entry.usdPrice;
                }
                return out;
            }, pairMints, 3_000);
        }
        logger.info({
            paperMode: this.env.paperMode,
            liveQuotes: this.env.liveQuotes,
            jito: this.env.jitoEnabled,
            monitor: this.env.monitorPort,
            scanInterval: this.env.scanIntervalMs,
            jupiterPollMints: (!this.env.paperMode || this.env.liveQuotes) ? pairMints.length : 0,
        }, 'bot: run loop started');
        let i = 0;
        while (this.running) {
            await this.scanOnce().catch((err) => {
                logger.error({ err }, 'bot: scanOnce error');
            });
            i += 1;
            if (maxIterations && i >= maxIterations)
                break;
            await new Promise((r) => setTimeout(r, this.env.scanIntervalMs));
        }
        this.pumpStrategy?.stop();
        this.opportunityDetector.stopAll();
        this.stopMonitor?.();
        logger.info(this.getStats(), 'bot: run loop stopped');
    }
    /**
     * Graceful shutdown — waits for in-flight trades to complete before stopping.
     * Called by SIGTERM/SIGINT handlers in index.ts.
     */
    async gracefulStop(timeoutMs = 10_000) {
        logger.info('bot: graceful stop requested');
        this.running = false;
        const deadline = Date.now() + timeoutMs;
        while (this.inFlightCount > 0 && Date.now() < deadline) {
            await new Promise((r) => setTimeout(r, 100));
        }
        if (this.inFlightCount > 0) {
            logger.warn({ inFlight: this.inFlightCount }, 'bot: timed out waiting for in-flight trades');
        }
        this.pumpStrategy?.stop();
        this.opportunityDetector.stopAll();
        this.stopMonitor?.();
    }
    stop() {
        this.running = false;
        this.pumpStrategy?.stop();
        this.opportunityDetector.stopAll();
        this.stopMonitor?.();
    }
}
