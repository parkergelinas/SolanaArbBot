/**
 * Multi-DEX arbitrage opportunity detector.
 *
 * Subscribes to price feeds from two or more DEXes simultaneously via
 * Connection.onAccountChange (Orca pools) and periodic Jupiter polling.
 * Calculates net profit after swap fees, Solana tx fees, and priority fees.
 * Filters out sub-threshold opportunities and ranks by profit-to-risk ratio.
 * Emits structured ArbOpportunity events.
 */
import { EventEmitter } from 'events';
export interface DexPriceFeed {
    dexName: string;
    /** Current price: token B per token A (UI amounts). */
    price: number;
    lastUpdatedMs: number;
}
export interface PoolSubscription {
    poolAddress: string;
    dexName: string;
    tokenMintA: string;
    tokenMintB: string;
    decimalsA: number;
    decimalsB: number;
}
export interface DetectorConfig {
    /** Minimum net profit in lamports to emit an opportunity. Default 0 (env: MIN_PROFIT_LAMPORTS). */
    minProfitLamports?: number;
    /** SOL price in USD used for fee conversion. Updated externally. */
    solPriceUsd?: number;
    /** Priority fee in lamports used for fee estimation. Default 50000. */
    priorityFeeLamports?: number;
    /** Max staleness of a price feed before ignoring it. Default 10000 ms. */
    maxPriceStaleMs?: number;
    /** DEX swap fee in bps for each leg. Default 30 (0.3%). */
    swapFeeBps?: number;
}
export declare class OpportunityDetector extends EventEmitter {
    private readonly connection;
    private readonly feeds;
    private readonly subscriptionIds;
    private readonly config;
    private jupiterPollTimer;
    private getJupiterPrices;
    constructor(rpcUrl: string, config?: DetectorConfig);
    updateSolPrice(usd: number): void;
    /** Register an on-chain Whirlpool (or any Orca-layout) pool for monitoring. */
    watchOrcaPool(sub: PoolSubscription): void;
    /** Register a Jupiter price polling feed for a pair. */
    startJupiterPolling(getPrices: (mints: string[]) => Promise<Record<string, number>>, pairsMintA: string[], intervalMs?: number): void;
    stopAll(): void;
    private handleOrcaAccountChange;
    private evaluatePair;
    private computeNetProfit;
}
