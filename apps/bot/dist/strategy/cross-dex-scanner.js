/**
 * Cross-DEX price scanner — compares Raydium vs Orca quotes for the same pair
 * using Jupiter's `dexes` parameter to force DEX-specific routing.
 *
 * Edge logic:
 *   1. Get quote for A→B restricted to Raydium pools only
 *   2. Get quote for A→B restricted to Orca/Whirlpool pools only
 *   3. Spread = abs(raydiumPrice - orcaPrice) / min(prices) * 10_000 (bps)
 *   4. Profitable if spread > combined fee floor (~30 bps round-trip)
 *
 * Execution model:
 *   Paper mode  — simulate both legs, record spread
 *   Live mode   — requires Jito bundle (two atomic txs) to avoid execution risk
 *                 between the buy-leg and sell-leg
 */
import { uiToAtomic } from '../jupiter/client.js';
// Jupiter DEX label groups
export const RAYDIUM_DEXES = 'Raydium,Raydium CLMM,Raydium CP';
export const ORCA_DEXES = 'Orca,Whirlpool';
// Combined fee floors in bps
// Raydium standard: 25 bps | Raydium CLMM: variable (4–100 bps, typical 4 bps)
// Orca Whirlpool: 5 bps
// Two-leg round-trip minimum: ~30 bps (buy Orca 5 bps + sell Raydium 25 bps)
export const CROSS_DEX_FEE_FLOOR_BPS = 30;
const QUOTE_TIMEOUT_MS = 8_000;
function withTimeout(p, ms, label) {
    return Promise.race([
        p,
        new Promise((_, rej) => setTimeout(() => rej(new Error(`timeout:${label}`)), ms)),
    ]);
}
/** Fetch a DEX-specific Jupiter quote; returns null on failure (no liquidity etc). */
async function fetchDexQuote(client, baseMint, quoteMint, amountAtomic, dexes, slippageBps) {
    try {
        return await withTimeout(client.getQuote({
            inputMint: baseMint,
            outputMint: quoteMint,
            amount: amountAtomic,
            slippageBps,
            restrictIntermediateTokens: true,
            dexes,
        }), QUOTE_TIMEOUT_MS, `${dexes.split(',')[0]}:${baseMint.slice(0, 8)}`);
    }
    catch {
        return null;
    }
}
/** Convert raw quote response to a normalised price (quote tokens per 1 base token). */
function quoteToPrice(quote, baseDecimals, quoteDecimals) {
    const inAtomic = Number(quote.inAmount);
    const outAtomic = Number(quote.outAmount);
    if (!inAtomic || !outAtomic)
        return 0;
    const inUi = inAtomic / 10 ** baseDecimals;
    const outUi = outAtomic / 10 ** quoteDecimals;
    return outUi / inUi;
}
/**
 * Scan a single pair for cross-DEX price divergence between Raydium and Orca.
 */
export async function scanCrossDexPair(client, pair, amountUi, slippageBps = 50) {
    const amountAtomic = uiToAtomic(amountUi, pair.baseDecimals).toString();
    const capturedAtMs = Date.now();
    const [raydiumQuote, orcaQuote] = await Promise.all([
        fetchDexQuote(client, pair.baseMint, pair.quoteMint, amountAtomic, RAYDIUM_DEXES, slippageBps),
        fetchDexQuote(client, pair.baseMint, pair.quoteMint, amountAtomic, ORCA_DEXES, slippageBps),
    ]);
    const raydiumPriceOut = raydiumQuote
        ? quoteToPrice(raydiumQuote, pair.baseDecimals, pair.quoteDecimals)
        : 0;
    const orcaPriceOut = orcaQuote
        ? quoteToPrice(orcaQuote, pair.baseDecimals, pair.quoteDecimals)
        : 0;
    let spreadBps = 0;
    let cheaperDex = 'equal';
    let dearerDex = 'equal';
    if (raydiumPriceOut > 0 && orcaPriceOut > 0) {
        const minPrice = Math.min(raydiumPriceOut, orcaPriceOut);
        spreadBps = Math.round(Math.abs(raydiumPriceOut - orcaPriceOut) / minPrice * 10_000);
        if (raydiumPriceOut > orcaPriceOut) {
            // Raydium gives more output → cheaper to buy base on Raydium (dearer = Orca)
            // Arb: buy on Raydium (more quote out), sell quote back on Orca
            cheaperDex = 'orca'; // sell base on Raydium (Raydium is dearer for base→quote)
            dearerDex = 'raydium';
            // Clarification: if Raydium gives MORE quote per base, Raydium is the BETTER sell venue.
            // To arb: buy base cheap (Orca gives less = base is cheaper there),
            //         sell base on Raydium (gives more quote).
            cheaperDex = 'orca'; // base is "cheaper" on Orca (you get less for it)
            dearerDex = 'raydium'; // base is "dearer" on Raydium (you get more for it → sell here)
        }
        else if (orcaPriceOut > raydiumPriceOut) {
            cheaperDex = 'raydium';
            dearerDex = 'orca';
        }
    }
    const netSpreadBps = spreadBps - CROSS_DEX_FEE_FLOOR_BPS;
    const profitable = netSpreadBps > 0 && raydiumPriceOut > 0 && orcaPriceOut > 0;
    return {
        pairLabel: pair.label,
        baseMint: pair.baseMint,
        quoteMint: pair.quoteMint,
        baseDecimals: pair.baseDecimals,
        quoteDecimals: pair.quoteDecimals,
        amountInUi: amountUi,
        raydiumQuote,
        orcaQuote,
        raydiumPriceOut,
        orcaPriceOut,
        spreadBps,
        netSpreadBps,
        cheaperDex,
        dearerDex,
        profitable,
        capturedAtMs,
    };
}
/**
 * Scan multiple pairs concurrently, respecting a concurrency limit.
 */
export async function scanCrossDexPairs(client, pairs, amountUi, opts = {}) {
    const slippageBps = opts.slippageBps ?? 50;
    const concurrency = opts.concurrency ?? 3;
    const opportunities = [];
    const errors = [];
    // Process in concurrency-limited batches
    for (let i = 0; i < pairs.length; i += concurrency) {
        const batch = pairs.slice(i, i + concurrency);
        const results = await Promise.allSettled(batch.map((p) => scanCrossDexPair(client, p, amountUi, slippageBps)));
        for (let j = 0; j < results.length; j++) {
            const r = results[j];
            if (r.status === 'fulfilled') {
                opportunities.push(r.value);
            }
            else {
                errors.push({
                    pairLabel: batch[j].label,
                    error: r.reason instanceof Error ? r.reason.message : String(r.reason),
                });
            }
        }
    }
    // Sort by spreadBps descending — widest spread first
    opportunities.sort((a, b) => b.spreadBps - a.spreadBps);
    return { opportunities, scannedAt: Date.now(), errors };
}
