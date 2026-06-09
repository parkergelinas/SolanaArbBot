/**
 * Strategy H: Cross-DEX Momentum Lag Capture
 *
 * When token X moves significantly on one DEX, competing DEXes often lag
 * in repricing due to AMM mechanics and low arbitrageur density on
 * smaller pairs.  This strategy:
 *
 *   1. Tracks price history per pair on Raydium and Orca independently.
 *   2. Computes a "momentum score": how much Raydium has moved vs Orca
 *      over the last MOMENTUM_WINDOW_MS.
 *   3. If Raydium has moved +X bps but Orca has only moved +Y bps (Y << X),
 *      the "lag" = X - Y bps.  We buy on Orca (lagging venue) expecting
 *      it to converge toward Raydium's price.
 *
 * This is a directional momentum strategy — we're not delta-neutral.
 * Paper mode: record the "expected" gain as the lag spread.
 * Risk: if Raydium's move was noise/reversal, we lose the lag spread.
 *
 * Pairs: SOL/USDC, WIF/USDC, BONK/USDC, JTO/USDC (liquid enough for tight
 *        slippage on both DEXes simultaneously).
 *
 * Parameters:
 *   minLagBps        — minimum Raydium-Orca price divergence to trade
 *   momentumWindowMs — look-back window for price history
 *   tradeSizeUsd     — notional trade size
 */

import type { JupiterClient } from '../jupiter/client.js';
import { uiToAtomic } from '../jupiter/client.js';
import type { MarketState } from '../market/state.js';
import type { ScannableStrategy } from '../signals/types.js';
import type { StrategyContext, TradeDecision } from './types.js';
import { estimateFeesUsd } from './scorer.js';
import {
  RAYDIUM_DEXES,
  ORCA_DEXES,
  type CrossDexScanPair,
} from './cross-dex-scanner.js';

const SOL_MINT  = 'So11111111111111111111111111111111111111112';
const USDC_MINT = 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v';
const WIF_MINT  = 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm';
const BONK_MINT = 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263';
const JTO_MINT  = 'jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL';

export const MOMENTUM_PAIRS: CrossDexScanPair[] = [
  { label: 'SOL/USDC',  baseMint: SOL_MINT,  quoteMint: USDC_MINT, baseDecimals: 9, quoteDecimals: 6 },
  { label: 'WIF/USDC',  baseMint: WIF_MINT,  quoteMint: USDC_MINT, baseDecimals: 6, quoteDecimals: 6 },
  { label: 'BONK/USDC', baseMint: BONK_MINT, quoteMint: USDC_MINT, baseDecimals: 5, quoteDecimals: 6 },
  { label: 'JTO/USDC',  baseMint: JTO_MINT,  quoteMint: USDC_MINT, baseDecimals: 9, quoteDecimals: 6 },
];

interface PriceSample {
  raydiumPrice: number;
  orcaPrice: number;
  timestampMs: number;
}

interface PairHistory {
  samples: PriceSample[];
}

const QUOTE_TIMEOUT_MS = 6_000;

function withTimeout<T>(p: Promise<T>, ms: number): Promise<T> {
  return Promise.race([
    p,
    new Promise<never>((_, rej) => setTimeout(() => rej(new Error('timeout')), ms)),
  ]);
}

async function fetchDexPrice(
  client: JupiterClient,
  pair: CrossDexScanPair,
  dexes: string,
  amountAtomic: string,
  slippageBps: number,
): Promise<number> {
  try {
    const q = await withTimeout(
      client.getQuote({
        inputMint: pair.baseMint,
        outputMint: pair.quoteMint,
        amount: amountAtomic,
        slippageBps,
        dexes,
      }),
      QUOTE_TIMEOUT_MS,
    );
    const inAtomic = Number(q.inAmount);
    const outAtomic = Number(q.outAmount);
    if (!inAtomic || !outAtomic) return 0;
    return (outAtomic / 10 ** pair.quoteDecimals) / (inAtomic / 10 ** pair.baseDecimals);
  } catch {
    return 0;
  }
}

export interface MomentumSignal {
  pairLabel: string;
  baseMint: string;
  quoteMint: string;
  baseDecimals: number;
  /** Current price on lagging DEX. */
  lagPrice: number;
  /** Current price on leading DEX. */
  leadPrice: number;
  lagDex: 'raydium' | 'orca';
  /** Lag spread in bps: lead-lag / lag. */
  lagSpreadBps: number;
  /** Momentum of the leading DEX over window (bps). */
  leadMomentumBps: number;
  capturedAtMs: number;
}

const MOMENTUM_KEY = '__momentum_signals';

export interface MomentumLagConfig {
  /** Minimum lag spread (bps) to signal a trade. */
  minLagBps: number;
  /** Price history window in ms — lag calculated over this period. */
  momentumWindowMs: number;
  /** Trade size in base token UI units. */
  tradeSizeUi: number;
  /** Min leading-DEX momentum to confirm trend (bps). */
  minLeadMomentumBps: number;
  concurrency: number;
}

export const DEFAULT_MOMENTUM_CONFIG: MomentumLagConfig = {
  minLagBps: 25,
  momentumWindowMs: 30_000,
  tradeSizeUi: 0.5,
  minLeadMomentumBps: 15,
  concurrency: 2,
};

export class MomentumLagStrategy implements ScannableStrategy {
  readonly id = 'momentum_lag_capture';

  /** Rolling price history per pair label. */
  private readonly history = new Map<string, PairHistory>();

  constructor(
    private readonly client: JupiterClient,
    private readonly cfg: MomentumLagConfig = DEFAULT_MOMENTUM_CONFIG,
  ) {}

  async scan(state: MarketState): Promise<MarketState> {
    const signals: MomentumSignal[] = [];
    const now = Date.now();

    for (let i = 0; i < MOMENTUM_PAIRS.length; i += this.cfg.concurrency) {
      const batch = MOMENTUM_PAIRS.slice(i, i + this.cfg.concurrency);
      await Promise.all(batch.map(async (pair) => {
        const amountAtomic = uiToAtomic(this.cfg.tradeSizeUi, pair.baseDecimals).toString();

        const [raydiumPrice, orcaPrice] = await Promise.all([
          fetchDexPrice(this.client, pair, RAYDIUM_DEXES, amountAtomic, 50),
          fetchDexPrice(this.client, pair, ORCA_DEXES, amountAtomic, 50),
        ]);

        if (raydiumPrice === 0 || orcaPrice === 0) return;

        // Update history
        let hist = this.history.get(pair.label);
        if (!hist) {
          hist = { samples: [] };
          this.history.set(pair.label, hist);
        }
        hist.samples.push({ raydiumPrice, orcaPrice, timestampMs: now });

        // Prune old samples
        const cutoff = now - this.cfg.momentumWindowMs * 2;
        hist.samples = hist.samples.filter((s) => s.timestampMs >= cutoff);

        // Need at least 2 samples to compute momentum
        if (hist.samples.length < 2) return;

        // Find the oldest sample within the momentum window
        const windowCutoff = now - this.cfg.momentumWindowMs;
        const windowSamples = hist.samples.filter((s) => s.timestampMs >= windowCutoff);
        if (windowSamples.length < 2) return;

        const oldest = windowSamples[0]!;
        const raydiumMomentumBps = Math.round(
          (raydiumPrice / oldest.raydiumPrice - 1) * 10_000,
        );
        const orcaMomentumBps = Math.round(
          (orcaPrice / oldest.orcaPrice - 1) * 10_000,
        );

        // Determine which DEX is leading (higher absolute momentum)
        const absRay = Math.abs(raydiumMomentumBps);
        const absOrca = Math.abs(orcaMomentumBps);
        if (absRay <= absOrca) return; // Orca leading or equal — skip

        // Raydium is leading; Orca is lagging
        const lagSpreadBps = Math.round(
          (raydiumPrice / orcaPrice - 1) * 10_000,
        );

        if (lagSpreadBps < this.cfg.minLagBps) return;
        if (absRay < this.cfg.minLeadMomentumBps) return;

        signals.push({
          pairLabel: pair.label,
          baseMint: pair.baseMint,
          quoteMint: pair.quoteMint,
          baseDecimals: pair.baseDecimals,
          lagPrice: orcaPrice,
          leadPrice: raydiumPrice,
          lagDex: 'orca',
          lagSpreadBps,
          leadMomentumBps: raydiumMomentumBps,
          capturedAtMs: now,
        });
      }));
    }

    signals.sort((a, b) => b.lagSpreadBps - a.lagSpreadBps);

    return { ...state, [MOMENTUM_KEY]: signals } as MarketState & Record<string, unknown>;
  }

  evaluate(state: MarketState, ctx: StrategyContext): TradeDecision | null {
    const signals: MomentumSignal[] = (state as unknown as Record<string, unknown>)[MOMENTUM_KEY] as MomentumSignal[] ?? [];
    if (signals.length === 0) return null;

    const best = signals[0]!;
    const solPrice = state.solPriceUsd || 150;
    const tokenPrice = state.pricesUsd[best.baseMint] ?? solPrice;
    const notionalUsd = this.cfg.tradeSizeUi * tokenPrice;

    const feesUsd = estimateFeesUsd({
      solPriceUsd: solPrice,
      slippageBps: ctx.slippageBps,
      notionalUsd,
    });

    const grossUsd = notionalUsd * (best.lagSpreadBps / 10_000);
    const netProfitUsd = grossUsd - feesUsd.totalUsd;

    const base: TradeDecision = {
      strategyId: this.id,
      pairLabel: best.pairLabel,
      inputMint: best.baseMint,
      outputMint: best.quoteMint,
      amountInAtomic: String(uiToAtomic(this.cfg.tradeSizeUi, best.baseDecimals)),
      expectedOutAtomic: String(Math.round(this.cfg.tradeSizeUi * best.leadPrice * 10 ** 6)),
      netProfitUsd,
      grossSpreadBps: best.lagSpreadBps,
      routeQualityScore: 0.75,
      freshnessScore: Math.max(0, 1 - (Date.now() - best.capturedAtMs) / 5000),
      metadata: {
        lagDex: best.lagDex,
        lagSpreadBps: best.lagSpreadBps,
        leadMomentumBps: best.leadMomentumBps,
        lagPrice: best.lagPrice,
        leadPrice: best.leadPrice,
      },
    };

    if (netProfitUsd < ctx.minProfitUsd) {
      return { ...base, rejectionReason: 'below_min_profit' };
    }

    return base;
  }
}
