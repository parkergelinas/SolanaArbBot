import type { JupiterClient } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import { type PumpEdgeConfig } from '../pump/edge-scorer.js';
import type { ScannableStrategy } from './types.js';
import type { StrategyContext, TradeDecision } from '../strategy/types.js';
/**
 * Pump.fun edge strategy — exploits bonding curve mispricing vs Jupiter.
 *
 * Edge sources (from pump-public-docs):
 * - Uniswap V2 bonding curve with 1% fee → predictable slippage
 * - Graduation at ~85 SOL → migration arb window to PumpSwap
 * - Fresh launches on curve before Jupiter routes exist
 */
export declare class PumpEdgeStrategy implements ScannableStrategy {
    private readonly client;
    private readonly rpcUrl;
    private readonly cfg;
    readonly id = "pump_edge";
    private readonly connection;
    private launchMonitor;
    private readonly recentLaunches;
    private readonly watchMints;
    constructor(client: JupiterClient, rpcUrl: string, cfg?: PumpEdgeConfig, heliusApiKey?: string);
    /** Add mints to watch for curve divergence (e.g. from pair registry). */
    watchMint(mint: string): void;
    stop(): void;
    scan(state: MarketState): Promise<MarketState>;
    evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null;
}
