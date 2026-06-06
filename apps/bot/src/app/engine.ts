import { loadEnv, SOL_MINT, USDC_MINT } from '../config/env.js';
import { logger } from '../logger.js';
import { getTotalPnlUsd } from '../db/sqlite.js';
import { HardenedExecutor } from '../execution/hardened-executor.js';
import { startMonitoringServer } from '../monitoring/server.js';
import { alertHalt, checkPnlAlert } from '../monitoring/alerts.js';

import { createMarketStack } from '../market-data/index.js';
import { pairRegistryFromEnv } from '../market/pair-registry.js';

import { computeDynamicSize } from '../sizing/dynamic.js';

import {
  checkStrategyRisk,
  DEFAULT_RISK_LIMITS,
  recordExecutionOutcome,
  type StrategyRiskState,
} from '../risk/index.js';

import {
  DEFAULT_MEAN_REVERSION_CONFIG,
  MeanReversionStrategy,
  PumpEdgeStrategy,
  RouteDivergenceArbStrategy,
  RoundTripQuoteArbStrategy,
  StrategyRegistry,
  type ScannableStrategy,
} from '../signals/index.js';

import type { Strategy } from '../signals/types.js';

import { TradeJournal } from '../state/journal.js';

import { computeAnalytics } from '../analytics/metrics.js';

import { LiveExecutor } from '../execution/index.js';

export interface EngineStats {
  scans: number;
  actionable: number;
  executed: number;
  rejected: number;
  pairCount: number;
}

/**
 * Main orchestrator — 4-layer pipeline:
 * MarketData → Signal → Risk → Execution
 */
export class BotEngine {
  private readonly env = loadEnv();
  private readonly journal = new TradeJournal();
  private readonly registry = new StrategyRegistry();
  private readonly stack = createMarketStack(this.env);
  private readonly pairRegistry = pairRegistryFromEnv(this.stack.client, this.env);
  private readonly executor: LiveExecutor;
  private readonly hardenedExecutor: HardenedExecutor;
  private stopMonitor: (() => void) | null = null;
  private inFlightCount = 0;
  private readonly scannable: ScannableStrategy[] = [];
  private pumpStrategy: PumpEdgeStrategy | null = null;

  private riskState: StrategyRiskState = {
    sessionLossUsd: 0,
    consecutiveQuoteFailures: 0,
    inventoryUsd: {},
    routeFamilyFailures: {},
    cooldownUntilMs: 0,
  };

  private stats: EngineStats = { scans: 0, actionable: 0, executed: 0, rejected: 0, pairCount: 0 };
  private failureRate = 0;
  private running = false;
  private tradeAmountUi: number;
  private initialized = false;

  constructor() {
    this.tradeAmountUi = this.env.tradeAmountUi;

    const routeDiv = new RouteDivergenceArbStrategy(
      this.stack.client,
      this.tradeAmountUi,
      this.pairRegistry,
      {
        minDivergenceBps: 12,
        minSurvivingEdgeBps: 8,
        pairsPerScan: this.env.pairsPerScan,
        scanConcurrency: 2,
      },
    );

    const roundTrip = new RoundTripQuoteArbStrategy(
      this.stack.client,
      this.tradeAmountUi,
      this.pairRegistry,
      { pairsPerScan: this.env.pairsPerScan, scanConcurrency: 3 },
    );

    const meanRev = new MeanReversionStrategy({
      ...DEFAULT_MEAN_REVERSION_CONFIG,
      enabled: this.env.enableMeanReversion,
    });

    this.scannable.push(routeDiv, roundTrip);
    this.registry.register(routeDiv);
    this.registry.register(roundTrip);
    this.registry.register(meanRev);

    if (this.env.enablePumpEdge) {
      this.pumpStrategy = new PumpEdgeStrategy(
        this.stack.client,
        this.env.rpcUrl,
        undefined,
        this.env.heliusApiKey,
      );
      this.scannable.push(this.pumpStrategy);
      this.registry.register(this.pumpStrategy);
    }

    this.executor = new LiveExecutor(this.env, this.stack.client, this.journal);

    this.hardenedExecutor = new HardenedExecutor(this.env, this.stack.client, this.journal, {
      jitoEnabled: this.env.jitoEnabled,
      jitoTipLamports: this.env.jitoTipLamports,
      minProfitLamports: this.env.minProfitLamports,
    });

    // Propagate dead-man's-switch halt to the engine loop
    this.hardenedExecutor.deadManSwitch.on('halt', async (evt) => {
      logger.error(evt, 'engine: dead-man-switch triggered halt');
      await alertHalt(evt.reason).catch(() => undefined);
      this.stop();
    });
  }

  getStats(): EngineStats {
    return { ...this.stats };
  }

  getJournal(): TradeJournal {
    return this.journal;
  }

  getAnalytics() {
    return computeAnalytics(this.journal.all());
  }

  getStrategies(): readonly Strategy[] {
    return this.registry.list();
  }

  getPairCount(): number {
    return this.pairRegistry.allPairs.length;
  }

  private async ensureInitialized(): Promise<void> {
    if (this.initialized) return;
    const pairs = await this.pairRegistry.refresh();
    this.stats.pairCount = pairs.length;

    if (this.pumpStrategy) {
      for (const meta of this.pairRegistry.pairMeta) {
        this.pumpStrategy.watchMint(meta.mint);
      }
    }

    logger.info(
      { pairs: pairs.length, source: this.env.pairSource, batch: this.env.pairsPerScan },
      'bot: pair universe loaded',
    );
    this.initialized = true;
  }

  async scanOnce(): Promise<void> {
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

    for (const strat of this.scannable) {
      state = await strat.scan(state);
    }

    const decision = this.pickDecision(state);

    if (!decision) return;

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

    // Use hardened executor in live mode, paper executor otherwise
    this.inFlightCount += 1;
    const result = this.env.paperMode
      ? await this.executor.execute(decision)
      : await this.hardenedExecutor.execute(decision);
    this.inFlightCount -= 1;

    this.riskState = recordExecutionOutcome(
      this.riskState,
      decision,
      result.realizedProfitUsd ?? 0,
      result.success,
    );

    if (result.success) {
      this.stats.executed += 1;
      logger.info(
        { pair: decision.pairLabel, profit: result.realizedProfitUsd, sig: result.signature },
        'bot: trade executed',
      );
      // Alert if session PnL drops below threshold
      await checkPnlAlert(getTotalPnlUsd(), state.solPriceUsd).catch(() => undefined);
    } else {
      this.stats.rejected += 1;
      this.failureRate = Math.min(1, this.failureRate + 0.05);
      logger.warn({ pair: decision.pairLabel, error: result.error }, 'bot: trade failed');
    }
  }

  private pickDecision(state: import('../market/state.js').MarketState) {
    const ctx = {
      minProfitUsd: this.env.minProfitUsd,
      slippageBps: this.env.slippageBps,
    };

    if (this.env.primaryStrategy === 'pump_edge' && this.pumpStrategy) {
      const pump = this.registry
        .evaluateAll(state, ctx)
        .find((d) => d.strategyId === 'pump_edge');
      if (pump && !pump.rejectionReason) return pump;
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

  async runLoop(maxIterations?: number): Promise<void> {
    this.running = true;

    // Start Express monitoring dashboard
    this.stopMonitor = startMonitoringServer({
      port: this.env.monitorPort,
      rpcUrl: this.env.rpcUrl,
      walletPublicKey: this.env.walletPublicKey,
      isHealthy: () => this.running && !this.hardenedExecutor.deadManSwitch.isHalted(),
    });

    logger.info(
      {
        paperMode: this.env.paperMode,
        jito: this.env.jitoEnabled,
        monitor: this.env.monitorPort,
        scanInterval: this.env.scanIntervalMs,
      },
      'bot: run loop started',
    );

    let i = 0;
    while (this.running) {
      await this.scanOnce().catch((err) => {
        logger.error({ err }, 'bot: scanOnce error');
      });
      i += 1;
      if (maxIterations && i >= maxIterations) break;
      await new Promise((r) => setTimeout(r, this.env.scanIntervalMs));
    }

    this.pumpStrategy?.stop();
    this.stopMonitor?.();
    logger.info(this.getStats(), 'bot: run loop stopped');
  }

  /**
   * Graceful shutdown — waits for in-flight trades to complete before stopping.
   * Called by SIGTERM/SIGINT handlers in index.ts.
   */
  async gracefulStop(timeoutMs = 10_000): Promise<void> {
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
    this.stopMonitor?.();
  }

  stop(): void {
    this.running = false;
    this.pumpStrategy?.stop();
    this.stopMonitor?.();
  }
}
