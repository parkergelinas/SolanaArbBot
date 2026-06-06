import { scanRoundTripQuotes } from './arb-scanner.js';
import { scanRouteDivergence } from './route-divergence-scanner.js';
async function mapWithConcurrency(items, concurrency, fn) {
    const results = [];
    for (let i = 0; i < items.length; i += concurrency) {
        const chunk = items.slice(i, i + concurrency);
        const chunkResults = await Promise.all(chunk.map(fn));
        results.push(...chunkResults);
    }
    return results;
}
/** Scan round-trip quotes across multiple pairs with concurrency control. */
export async function scanMultipleRoundTrips(client, pairs, tradeAmountUi, opts = {}) {
    const concurrency = opts.concurrency ?? 3;
    const quotes = new Map();
    const divergences = new Map();
    const errors = new Map();
    const results = await mapWithConcurrency(pairs, concurrency, async (pair) => {
        try {
            const quote = await scanRoundTripQuotes(client, pair, tradeAmountUi, opts);
            return { pair, quote, error: null };
        }
        catch (err) {
            return {
                pair,
                quote: null,
                error: err instanceof Error ? err.message : String(err),
            };
        }
    });
    for (const r of results) {
        if (r.quote)
            quotes.set(r.pair.label, r.quote);
        if (r.error)
            errors.set(r.pair.label, r.error);
    }
    return { quotes, divergences, errors };
}
/** Scan route divergence across multiple pairs. */
export async function scanMultipleRouteDivergences(client, pairs, tradeAmountUi, opts = {}) {
    const concurrency = opts.concurrency ?? 2;
    const quotes = new Map();
    const divergences = new Map();
    const errors = new Map();
    const results = await mapWithConcurrency(pairs, concurrency, async (pair) => {
        try {
            const div = await scanRouteDivergence(client, pair, tradeAmountUi, opts);
            return { pair, div, error: null };
        }
        catch (err) {
            return {
                pair,
                div: null,
                error: err instanceof Error ? err.message : String(err),
            };
        }
    });
    for (const r of results) {
        if (r.div)
            divergences.set(r.pair.label, r.div);
        if (r.error)
            errors.set(r.pair.label, r.error);
    }
    return { quotes, divergences, errors };
}
