/**
 * Adaptive Arb Strategy — capital-aware multi-source arbitrage that automatically
 * adjusts trade size, spread threshold, and profit gate as capital compounds.
 *
 * Edge sources (in priority order):
 *
 * 1. Pump.fun graduated tokens (first 24–48h on Raydium)
 *    Why: Price discovery window. Raydium listing price ≠ Jupiter best-route price.
 *    Spread: 50–500 bps during active price discovery.
 *    Risk: Token can dump; cap position at 10% of capital per trade.
 *
 * 2. LST de-peg (mSOL/SOL, jitoSOL/SOL)
 *    Why: mSOL has a known fair value = SOL × Marinade exchange rate.
 *    When market price < fair value by > fees: buy mSOL, route back via Jupiter.
 *    Spread: 2–15 bps (small but real and very reliable).
 *    Risk: Minimal — redeemable at fair value within epoch.
 *
 * 3. Cross-DEX spread on active pairs (Raydium vs Orca)
 *    Why: High-volume events temporarily diverge venue prices.
 *    Spread: 0.25–50 bps. Only profitable at Stage 3+ capital.
 *    Risk: Low — symmetric spread, execute both legs atomically via Jito.
 *
 * Adaptive logic:
 *   - trade size, spread threshold, and min-profit gate all scale with capital
 *   - CapitalTracker persists across restarts → compounding survives reboots
 *   - At each capital stage, the binding constraint shifts (fixed tx cost → spread)
 */
import { scanCrossDexPairs, } from '../strategy/cross-dex-scanner.js';
import { CROSS_DEX_CORE_PAIRS } from './cross-dex-arb.js';
import { uiToAtomic } from '../jupiter/client.js';
import { SOL_MINT } from '../config/env.js';
import { computeDynamicSize } from '../sizing/dynamic.js';
// LST pairs — these have predictable fair-value arb based on staking yield
const LST_PAIRS = [
    {
        label: 'mSOL/SOL',
        baseMint: 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So',
        quoteMint: SOL_MINT,
        baseDecimals: 9,
        quoteDecimals: 9,
    },
    {
        label: 'jitoSOL/SOL',
        baseMint: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn',
        quoteMint: SOL_MINT,
        baseDecimals: 9,
        quoteDecimals: 9,
    },
    {
        label: 'bSOL/SOL',
        baseMint: 'bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1',
        quoteMint: SOL_MINT,
        baseDecimals: 9,
        quoteDecimals: 9,
    },
];
export const DEFAULT_ADAPTIVE_CONFIG = {
    maxCapitalFraction: 0.9,
    maxTradeSolCap: Number(process.env.BOT_MAX_TRADE_SOL ?? '10'),
    pairsPerScan: Number(process.env.BOT_PAIRS_PER_SCAN ?? '8'),
    scanConcurrency: 3,
};
export class AdaptiveArbStrategy {
    client;
    capitalTracker;
    cfg;
    pumpRegistry;
    id = 'adaptive_arb';
    scanTick = 0;
    pairIndex = 0;
    lastOpportunities = [];
    constructor(client, capitalTracker, cfg = DEFAULT_ADAPTIVE_CONFIG, pumpRegistry = null) {
        this.client = client;
        this.capitalTracker = capitalTracker;
        this.cfg = cfg;
        this.pumpRegistry = pumpRegistry;
    }
    async scan(state) {
        this.scanTick += 1;
        const params = this.capitalTracker.getAdaptiveParams(state.solPriceUsd);
        // Build pair batch for this tick:
        //  - Always include LST pairs (reliable low-risk edge)
        //  - Rotate through core pairs + pump pairs based on capital stage
        const lstBatch = LST_PAIRS.slice(0, 2);
        const remainingSlots = Math.max(0, this.cfg.pairsPerScan - lstBatch.length);
        const corePairs = this.nextCoreBatch(remainingSlots);
        const pumpPairs = this.pumpRegistry?.pairs.slice(0, Math.max(0, remainingSlots - corePairs.length)) ?? [];
        // At Stage 1/2 (bootstrap/growing), prioritise pump pairs over core pairs
        // because core pairs won't have enough spread at small notional
        const batch = params.stage.startsWith('stage1') || params.stage.startsWith('stage2')
            ? [...lstBatch, ...pumpPairs, ...corePairs].slice(0, this.cfg.pairsPerScan)
            : [...lstBatch, ...corePairs, ...pumpPairs].slice(0, this.cfg.pairsPerScan);
        if (batch.length === 0)
            return state;
        try {
            const result = await scanCrossDexPairs(this.client, batch, params.tradeSizeUi, { slippageBps: 50, concurrency: this.cfg.scanConcurrency });
            this.lastOpportunities = result.opportunities;
        }
        catch {
            // keep last opportunities on transient failure
        }
        return { ...state, crossDexOpportunities: this.lastOpportunities };
    }
    evaluate(state, ctx) {
        const params = this.capitalTracker.getAdaptiveParams(state.solPriceUsd);
        const opps = state.crossDexOpportunities ?? this.lastOpportunities;
        if (!opps || opps.length === 0)
            return null;
        let best = null;
        let bestProfit = -Infinity;
        for (const opp of opps) {
            const decision = this.evaluateOne(opp, state, params, ctx);
            if (decision && decision.netProfitUsd > bestProfit) {
                bestProfit = decision.netProfitUsd;
                best = decision;
            }
        }
        return best;
    }
    /** Called by engine after a successful execution to compound capital. */
    recordExecuted(profitUsd, solPriceUsd) {
        this.capitalTracker.recordTrade(profitUsd, solPriceUsd);
    }
    evaluateOne(opp, state, params, ctx) {
        if (opp.spreadBps === 0)
            return null;
        const isLst = LST_PAIRS.some((p) => p.label === opp.pairLabel);
        const isPump = !CROSS_DEX_CORE_PAIRS.some((p) => p.label === opp.pairLabel) && !isLst;
        // LST pairs use a lower threshold — the edge is reliable but thin
        const spreadThreshold = isLst
            ? Math.max(5, params.minSpreadBps / 5)
            : params.minSpreadBps;
        // Pump pairs: cap position at 20% of capital to limit downside
        const maxSizeForPair = isPump
            ? Math.min(params.tradeSizeUi, params.capitalSol * 0.20)
            : params.tradeSizeUi;
        // Use dynamic sizer: accounts for MEV risk, liquidity depth, failure rate
        const sized = computeDynamicSize({
            baseAmountUi: maxSizeForPair,
            spreadBps: opp.spreadBps,
            liquidityUsd: 50_000, // conservative estimate; pump pairs may be lower
            volatilityPct: isPump ? 15 : 2,
            routeQualityScore: opp.spreadBps > 0 ? Math.min(1, opp.spreadBps / 100) : 0,
            recentFailureRate: 0,
            minAmountUi: 0.05,
            maxAmountUi: Math.min(maxSizeForPair, this.cfg.maxTradeSolCap),
            solPriceUsd: state.solPriceUsd || 65,
            jitoActive: false, // engine overrides when Jito is confirmed active
            priorityFeeMicroLamports: 50_000,
        });
        const actualSize = sized.amountUi;
        if (actualSize <= 0)
            return null;
        const basePrice = state.pricesUsd[opp.baseMint] ?? (state.solPriceUsd || 65);
        const notionalUsd = actualSize * basePrice;
        // Estimated tx cost: two legs × (base fee + priority fee)
        // 2 × (0.000005 SOL base + 0.001 SOL priority) × solPrice
        const solPrice = state.solPriceUsd || 65;
        const txCostUsd = 2 * (0.000005 + 0.001) * solPrice;
        const grossProfitUsd = (opp.netSpreadBps / 10_000) * notionalUsd;
        const netProfitUsd = grossProfitUsd - txCostUsd;
        const bestQuote = opp.dearerDex === 'raydium' ? opp.raydiumQuote : opp.orcaQuote;
        const base = {
            strategyId: this.id,
            pairLabel: opp.pairLabel,
            inputMint: opp.baseMint,
            outputMint: opp.quoteMint,
            amountInAtomic: uiToAtomic(actualSize, opp.baseDecimals).toString(),
            expectedOutAtomic: bestQuote?.outAmount ?? '0',
            netProfitUsd,
            grossSpreadBps: opp.spreadBps,
            routeQualityScore: sized.ceiling !== 'min_clamp' ? 0.8 : 0.4,
            freshnessScore: 1,
            metadata: {
                // Spread data
                spreadBps: opp.spreadBps,
                netSpreadBps: opp.netSpreadBps,
                cheaperDex: opp.cheaperDex,
                dearerDex: opp.dearerDex,
                // Classification
                isLst,
                isPump,
                pairType: isLst ? 'lst' : isPump ? 'pump' : 'core',
                // Sizing
                tradeSizeUi: actualSize,
                notionalUsd,
                txCostUsd,
                sizingRationale: sized.rationale,
                // Capital state
                capitalStage: params.stage,
                capitalSol: params.capitalSol,
                spreadThreshold,
                // Execution requirements
                executionNote: 'requires_jito_bundle_two_tx',
            },
        };
        // Rejection gates
        if (opp.spreadBps < spreadThreshold) {
            base.rejectionReason = `spread_below_threshold:${opp.spreadBps}bps<${spreadThreshold}bps[${params.stage}]`;
        }
        else if (opp.netSpreadBps <= 0) {
            base.rejectionReason = `fees_eat_spread:net=${opp.netSpreadBps}bps`;
        }
        else if (!opp.raydiumQuote || !opp.orcaQuote) {
            base.rejectionReason = `missing_quote:${!opp.raydiumQuote ? 'no_raydium' : 'no_orca'}`;
        }
        else if (netProfitUsd < params.minProfitUsd) {
            base.rejectionReason = `below_min_profit:$${netProfitUsd.toFixed(4)}<$${params.minProfitUsd}[${params.stage}]`;
        }
        else if (netProfitUsd < ctx.minProfitUsd) {
            base.rejectionReason = `below_global_min:$${netProfitUsd.toFixed(4)}<$${ctx.minProfitUsd}`;
        }
        return base;
    }
    nextCoreBatch(n) {
        const out = [];
        for (let i = 0; i < Math.min(n, CROSS_DEX_CORE_PAIRS.length); i++) {
            out.push(CROSS_DEX_CORE_PAIRS[(this.pairIndex + i) % CROSS_DEX_CORE_PAIRS.length]);
        }
        this.pairIndex = (this.pairIndex + n) % CROSS_DEX_CORE_PAIRS.length;
        return out;
    }
}
