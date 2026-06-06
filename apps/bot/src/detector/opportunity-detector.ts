/**
 * Multi-DEX arbitrage opportunity detector.
 *
 * Subscribes to price feeds from two or more DEXes simultaneously via
 * Connection.onAccountChange (Orca pools) and periodic Jupiter polling.
 * Calculates net profit after swap fees, Solana tx fees, and priority fees.
 * Filters out sub-threshold opportunities and ranks by profit-to-risk ratio.
 * Emits structured ArbOpportunity events.
 */

import { Connection, PublicKey, type AccountInfo } from '@solana/web3.js';
import { EventEmitter } from 'events';
import { logger } from '../logger.js';
import { sqrtPriceX64ToUiPrice } from '../orca/whirlpool-monitor.js';
import type { ArbOpportunity } from './types.js';

const LAMPORTS_PER_SOL = 1_000_000_000;
/** Base Solana tx fee in lamports (5000 × 1 signature, rounded up). */
const BASE_TX_FEE_LAMPORTS = 5_000;

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

export class OpportunityDetector extends EventEmitter {
  private readonly connection: Connection;
  private readonly feeds: Map<string, Map<string, DexPriceFeed>> = new Map();
  // feeds keyed by `${mintA}:${mintB}` → map of dexName → DexPriceFeed
  private readonly subscriptionIds: Map<string, number> = new Map();
  private readonly config: Required<DetectorConfig>;
  private jupiterPollTimer: ReturnType<typeof setInterval> | null = null;
  private getJupiterPrices: ((mints: string[]) => Promise<Record<string, number>>) | null = null;

  constructor(rpcUrl: string, config: DetectorConfig = {}) {
    super();
    this.connection = new Connection(rpcUrl, 'processed');
    this.config = {
      minProfitLamports: config.minProfitLamports
        ?? Number(process.env.MIN_PROFIT_LAMPORTS ?? '0'),
      solPriceUsd: config.solPriceUsd ?? 170,
      priorityFeeLamports: config.priorityFeeLamports ?? 50_000,
      maxPriceStaleMs: config.maxPriceStaleMs ?? 10_000,
      swapFeeBps: config.swapFeeBps ?? 30,
    };
  }

  updateSolPrice(usd: number): void {
    this.config.solPriceUsd = usd;
  }

  /** Register an on-chain Whirlpool (or any Orca-layout) pool for monitoring. */
  watchOrcaPool(sub: PoolSubscription): void {
    const pairKey = `${sub.tokenMintA}:${sub.tokenMintB}`;
    if (!this.feeds.has(pairKey)) this.feeds.set(pairKey, new Map());

    if (this.subscriptionIds.has(sub.poolAddress)) return;

    const pubkey = new PublicKey(sub.poolAddress);
    const id = this.connection.onAccountChange(
      pubkey,
      (info) => this.handleOrcaAccountChange(info, sub, pairKey),
      'processed',
    );
    this.subscriptionIds.set(sub.poolAddress, id);
    logger.info({ pool: sub.poolAddress, dex: sub.dexName }, 'detector: watching pool');
  }

  /** Register a Jupiter price polling feed for a pair. */
  startJupiterPolling(
    getPrices: (mints: string[]) => Promise<Record<string, number>>,
    pairsMintA: string[],
    intervalMs = 2_000,
  ): void {
    this.getJupiterPrices = getPrices;
    if (this.jupiterPollTimer) clearInterval(this.jupiterPollTimer);

    this.jupiterPollTimer = setInterval(async () => {
      try {
        const prices = await getPrices(pairsMintA);
        for (const [mint, price] of Object.entries(prices)) {
          if (price <= 0) continue;
          // Jupiter prices are in USD; they represent one token in USD.
          // For cross-DEX comparison, we store as price of mintA in USDC.
          const pairKey = `${mint}:EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`;
          if (!this.feeds.has(pairKey)) this.feeds.set(pairKey, new Map());
          this.feeds.get(pairKey)!.set('jupiter', {
            dexName: 'jupiter',
            price,
            lastUpdatedMs: Date.now(),
          });
          this.evaluatePair(mint, 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', pairKey);
        }
      } catch (err) {
        logger.debug({ err }, 'detector: jupiter poll failed');
      }
    }, intervalMs);
  }

  stopAll(): void {
    if (this.jupiterPollTimer) {
      clearInterval(this.jupiterPollTimer);
      this.jupiterPollTimer = null;
    }
    for (const [addr, id] of this.subscriptionIds) {
      this.connection.removeAccountChangeListener(id).catch(() => undefined);
      logger.debug({ addr }, 'detector: unsubscribed');
    }
    this.subscriptionIds.clear();
  }

  private handleOrcaAccountChange(
    info: AccountInfo<Buffer>,
    sub: PoolSubscription,
    pairKey: string,
  ): void {
    try {
      const sqrtPriceX64 = readU128LE(info.data, 65);
      const price = sqrtPriceX64ToUiPrice(sqrtPriceX64, sub.decimalsA, sub.decimalsB);
      if (price <= 0) return;

      const feed: DexPriceFeed = {
        dexName: sub.dexName,
        price,
        lastUpdatedMs: Date.now(),
      };
      this.feeds.get(pairKey)!.set(sub.dexName, feed);
      this.evaluatePair(sub.tokenMintA, sub.tokenMintB, pairKey);
    } catch (err) {
      logger.debug({ err, pool: sub.poolAddress }, 'detector: decode error');
    }
  }

  private evaluatePair(mintA: string, mintB: string, pairKey: string): void {
    const feedMap = this.feeds.get(pairKey);
    if (!feedMap || feedMap.size < 2) return;

    const now = Date.now();
    const active = [...feedMap.values()].filter(
      (f) => now - f.lastUpdatedMs < this.config.maxPriceStaleMs,
    );
    if (active.length < 2) return;

    // Find the best arb pair: cheapest and most expensive
    active.sort((a, b) => a.price - b.price);
    const cheapest = active[0];
    const priciest = active[active.length - 1];

    const grossSpreadBps = Math.round(
      ((priciest.price - cheapest.price) / cheapest.price) * 10_000,
    );

    const opportunity = this.computeNetProfit(
      mintA,
      mintB,
      cheapest,
      priciest,
      grossSpreadBps,
    );

    if (opportunity === null) return;

    if (opportunity.profitLamports < this.config.minProfitLamports) {
      logger.debug(
        { profit: opportunity.profitLamports, min: this.config.minProfitLamports },
        'detector: opportunity below min profit threshold',
      );
      return;
    }

    this.emit('opportunity', opportunity);
    logger.info(opportunity, 'detector: arb opportunity');
  }

  private computeNetProfit(
    mintA: string,
    mintB: string,
    cheapFeed: DexPriceFeed,
    expFeed: DexPriceFeed,
    grossSpreadBps: number,
  ): ArbOpportunity | null {
    if (cheapFeed.price <= 0) return null;

    // Assume $1 notional for fee ratio calculations
    const notionalUsd = this.config.solPriceUsd; // trade ~1 SOL worth
    const grossProfitUsd = notionalUsd * (grossSpreadBps / 10_000);

    // Swap fees: 2 legs × swapFeeBps
    const swapFeesUsd = notionalUsd * ((this.config.swapFeeBps * 2) / 10_000);
    // Solana base + priority fees
    const txFeeLamports = BASE_TX_FEE_LAMPORTS + this.config.priorityFeeLamports;
    const txFeeUsd = (txFeeLamports / LAMPORTS_PER_SOL) * this.config.solPriceUsd;
    const totalFeesUsd = swapFeesUsd + txFeeUsd;

    const netProfitUsd = grossProfitUsd - totalFeesUsd;
    const netProfitLamports = Math.floor(
      (netProfitUsd / this.config.solPriceUsd) * LAMPORTS_PER_SOL,
    );

    if (netProfitUsd <= 0) return null;

    // Risk proxy: inverse of spread stability (larger spread = higher risk of price move)
    const riskScore = Math.max(1, grossSpreadBps);
    const profitToRiskRatio = netProfitUsd / riskScore;

    return {
      tokenIn: mintA,
      tokenOut: mintB,
      dex1: cheapFeed.dexName,
      dex2: expFeed.dexName,
      profitUsd: netProfitUsd,
      profitLamports: netProfitLamports,
      profitToRiskRatio,
      grossSpreadBps,
      feesUsd: {
        swapFeesUsd,
        txFeeLamports,
        priorityFeeLamports: this.config.priorityFeeLamports,
        totalUsd: totalFeesUsd,
      },
      route: [mintA, mintB, mintA],
      detectedAtMs: Date.now(),
      dex1Price: cheapFeed.price,
      dex2Price: expFeed.price,
    };
  }
}

// ─── helpers ──────────────────────────────────────────────────────────────────

function readU128LE(buf: Buffer, offset: number): bigint {
  let result = 0n;
  for (let i = 0; i < 16; i++) {
    result |= BigInt(buf[offset + i]) << BigInt(8 * i);
  }
  return result;
}
