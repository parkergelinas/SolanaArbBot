import type { JupiterClient } from '../jupiter/client.js';
import type { QuotePairSnapshot } from '../jupiter/types.js';
import { scanRoundTripQuotes, type ArbScanOptions, type ScanPair } from './arb-scanner.js';
import { scanRouteDivergence } from './route-divergence-scanner.js';
import type { RouteDivergenceSnapshot } from '../market/state.js';

export interface MultiPairScanResult {
  quotes: Map<string, QuotePairSnapshot>;
  divergences: Map<string, RouteDivergenceSnapshot>;
  errors: Map<string, string>;
}

export interface MultiPairScanOptions extends ArbScanOptions {
  /** Max concurrent Jupiter quote requests. */
  concurrency?: number;
  /** Delay between batches (ms) to respect rate limits. */
  batchDelayMs?: number;
}

async function mapWithConcurrency<T, R>(
  items: T[],
  concurrency: number,
  fn: (item: T) => Promise<R>,
  batchDelayMs = 0,
): Promise<R[]> {
  const results: R[] = [];
  for (let i = 0; i < items.length; i += concurrency) {
    if (i > 0 && batchDelayMs > 0) {
      await new Promise((r) => setTimeout(r, batchDelayMs));
    }
    const chunk = items.slice(i, i + concurrency);
    const chunkResults = await Promise.all(chunk.map(fn));
    results.push(...chunkResults);
  }
  return results;
}

/** Scan round-trip quotes across multiple pairs with concurrency control. */
export async function scanMultipleRoundTrips(
  client: JupiterClient,
  pairs: ScanPair[],
  tradeAmountUi: number,
  opts: MultiPairScanOptions = {},
): Promise<MultiPairScanResult> {
  const concurrency = opts.concurrency ?? 3;
  const quotes = new Map<string, QuotePairSnapshot>();
  const divergences = new Map<string, RouteDivergenceSnapshot>();
  const errors = new Map<string, string>();

  const results = await mapWithConcurrency(pairs, concurrency, async (pair) => {
    try {
      const quote = await scanRoundTripQuotes(client, pair, tradeAmountUi, opts);
      return { pair, quote, error: null as string | null };
    } catch (err) {
      return {
        pair,
        quote: null as QuotePairSnapshot | null,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }, opts.batchDelayMs ?? 0);

  for (const r of results) {
    if (r.quote) quotes.set(r.pair.label, r.quote);
    if (r.error) errors.set(r.pair.label, r.error);
  }

  return { quotes, divergences, errors };
}

/** Scan route divergence across multiple pairs. */
export async function scanMultipleRouteDivergences(
  client: JupiterClient,
  pairs: ScanPair[],
  tradeAmountUi: number,
  opts: MultiPairScanOptions = {},
): Promise<MultiPairScanResult> {
  const concurrency = opts.concurrency ?? 2;
  const quotes = new Map<string, QuotePairSnapshot>();
  const divergences = new Map<string, RouteDivergenceSnapshot>();
  const errors = new Map<string, string>();

  const results = await mapWithConcurrency(pairs, concurrency, async (pair) => {
    try {
      const div = await scanRouteDivergence(client, pair, tradeAmountUi, opts);
      return { pair, div, error: null as string | null };
    } catch (err) {
      return {
        pair,
        div: null as RouteDivergenceSnapshot | null,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }, opts.batchDelayMs ?? 0);

  for (const r of results) {
    if (r.div) divergences.set(r.pair.label, r.div);
    if (r.error) errors.set(r.pair.label, r.error);
  }

  return { quotes, divergences, errors };
}
