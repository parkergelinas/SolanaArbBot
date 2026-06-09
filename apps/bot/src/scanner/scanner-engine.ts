import { createMarketStack } from '../market-data/index.js';
import { pairRegistryFromEnv } from '../market/pair-registry.js';
import { loadEnv, SOL_MINT, USDC_MINT } from '../config/env.js';
import {
  RouteDivergenceArbStrategy,
  CrossDexArbStrategy,
  DEFAULT_CROSS_DEX_CONFIG,
  type ScannableStrategy,
} from '../signals/index.js';
import type { TradeDecision, StrategyContext } from '../strategy/types.js';
import type { MarketState } from '../market/state.js';

export interface ScanResult {
  strategyId: string;
  pairLabel: string;
  spreadBps: number;
  netProfitUsd: number;
  score: number;
  simPass: boolean;
  survival200ms: boolean;
  capitalUsd: number;
  timestamp: number;
  decision: TradeDecision;
}

export class ScannerEngine {
  private readonly env = loadEnv();
  private readonly stack = createMarketStack(this.env);
  private readonly pairRegistry = pairRegistryFromEnv(this.stack.client, this.env);
  private readonly ctx: StrategyContext;
  private readonly strategies: ScannableStrategy[];
  private readonly capitalUsd: number;
  private initialized = false;

  constructor(capitalUsd: number) {
    this.capitalUsd = capitalUsd;
    this.ctx = { minProfitUsd: 0, slippageBps: this.env.slippageBps };

    const tradeAmountUi = capitalUsd / 150; // 150 USD/SOL bootstrap estimate

    const routeDiv = new RouteDivergenceArbStrategy(
      this.stack.client,
      tradeAmountUi,
      this.pairRegistry,
      {
        minDivergenceBps: Number(process.env.BOT_MIN_DIVERGENCE_BPS ?? '1'),
        minSurvivingEdgeBps: Number(process.env.BOT_MIN_SURVIVING_EDGE_BPS ?? '1'),
        pairsPerScan: this.env.pairsPerScan,
        scanConcurrency: 2,
      },
      true, // paperMode always on in scanner
      true, // liveQuotes always on in scanner
    );

    const crossDex = new CrossDexArbStrategy(
      this.stack.client,
      tradeAmountUi,
      DEFAULT_CROSS_DEX_CONFIG,
      null,
    );

    this.strategies = [routeDiv, crossDex];
  }

  async init(): Promise<void> {
    await this.pairRegistry.refresh();
    this.initialized = true;
  }

  async scan(): Promise<ScanResult[]> {
    if (!this.initialized) await this.init();

    const baseState: MarketState = {
      timestampMs: Date.now(),
      pricesUsd: { [SOL_MINT]: 150, [USDC_MINT]: 1 },
      decimals: { [SOL_MINT]: 9, [USDC_MINT]: 6 },
      quality: {},
      universe: [SOL_MINT, USDC_MINT],
      solPriceUsd: 150,
    };

    const results: ScanResult[] = [];

    for (const strategy of this.strategies) {
      const scanStart = Date.now();
      try {
        const enrichedState = await strategy.scan(baseState);
        const decision = strategy.evaluate(enrichedState, this.ctx);
        if (!decision) continue;

        const elapsed = Date.now() - scanStart;
        results.push({
          strategyId: strategy.id,
          pairLabel: decision.pairLabel,
          spreadBps: decision.grossSpreadBps,
          netProfitUsd: decision.netProfitUsd,
          score: decision.routeQualityScore,
          simPass: decision.netProfitUsd > 0,
          survival200ms: elapsed < 200,
          capitalUsd: this.capitalUsd,
          timestamp: Date.now(),
          decision,
        });
      } catch {
        // individual strategy errors don't stop the scan cycle
      }
    }

    return results;
  }

  stop(): void {
    // interval ownership lives in live-collector; nothing to clean up here
  }
}
