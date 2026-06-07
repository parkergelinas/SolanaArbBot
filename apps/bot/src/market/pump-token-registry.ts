/**
 * Pump.fun & memecoin token registry — discovers high-spread Solana tokens
 * using DexScreener's free API.
 *
 * Target tokens:
 *   - Graduated pump.fun tokens (now on Raydium, still volatile)
 *   - Any Solana token with wide Raydium↔Orca price spread
 *   - Low-to-mid cap ($10k–$10M) — large caps are efficiently arb'd already
 *
 * Refresh strategy:
 *   - Full refresh every REFRESH_INTERVAL_MS (default 5 minutes)
 *   - Token list is cached between refreshes
 *   - Pairs are fed into CrossDexArbStrategy for spread measurement
 */

import type { CrossDexScanPair } from '../strategy/cross-dex-scanner.js';
import { USDC_MINT } from '../config/env.js';

const DS_BASE = 'https://api.dexscreener.com';
const TIMEOUT_MS = 8_000;
const REFRESH_INTERVAL_MS = 5 * 60 * 1_000; // 5 minutes

// Target market cap range — below this, too illiquid; above, too efficiently arb'd
const MIN_LIQUIDITY_USD = 5_000;
const MAX_LIQUIDITY_USD = 2_000_000;

// Minimum 24h volume — ensures enough activity to actually trade
const MIN_VOLUME_24H_USD = 3_000;

// Max age of pair listing to consider (7 days) — newer = more likely to be mispriced
const MAX_PAIR_AGE_DAYS = 7;

// Maximum tokens to track at once
const MAX_TRACKED = 30;

interface DsPair {
  pairAddress?: string;
  baseToken?: { address?: string; symbol?: string; name?: string };
  quoteToken?: { address?: string; symbol?: string };
  priceUsd?: string;
  liquidity?: { usd?: number };
  volume?: { h24?: number };
  priceChange?: { h1?: number; h24?: number };
  pairCreatedAt?: number; // unix ms
  dexId?: string;
  chainId?: string;
}

interface DsSearchResult {
  pairs?: DsPair[];
}

async function fetchJson<T>(url: string): Promise<T | null> {
  try {
    const resp = await fetch(url, {
      headers: { Accept: 'application/json' },
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
    if (!resp.ok) return null;
    return (await resp.json()) as T;
  } catch {
    return null;
  }
}

/**
 * Score a pair for arb potential — higher = more likely to have wide spread.
 * Factors: volatility (price change), recency, volume, liquidity in target range.
 */
function scorePair(pair: DsPair): number {
  const vol24h = pair.volume?.h24 ?? 0;
  const liq = pair.liquidity?.usd ?? 0;
  const change1h = Math.abs(pair.priceChange?.h1 ?? 0);
  const change24h = Math.abs(pair.priceChange?.h24 ?? 0);
  const ageMs = pair.pairCreatedAt ? Date.now() - pair.pairCreatedAt : Infinity;
  const ageDays = ageMs / (1_000 * 60 * 60 * 24);

  if (liq < MIN_LIQUIDITY_USD || liq > MAX_LIQUIDITY_USD) return 0;
  if (vol24h < MIN_VOLUME_24H_USD) return 0;
  if (ageDays > MAX_PAIR_AGE_DAYS) return 0;

  // Higher score = more likely mispriced
  const recencyBonus = Math.max(0, (MAX_PAIR_AGE_DAYS - ageDays) / MAX_PAIR_AGE_DAYS);
  const volatilityScore = Math.min(100, change1h * 2 + change24h * 0.5);
  const volumeScore = Math.min(100, Math.log10(vol24h + 1) * 10);

  return recencyBonus * 40 + volatilityScore * 40 + volumeScore * 20;
}

/** Known decimals for common quote tokens; defaults to 6 for unknown mints. */
function quoteDecimals(mint: string): number {
  // USDC, USDT, and most SPL tokens use 6 decimals
  return 6;
}

/** Convert a DexScreener pair to a CrossDexScanPair. */
function toCrossDexPair(pair: DsPair): CrossDexScanPair | null {
  const base = pair.baseToken?.address;
  const sym = pair.baseToken?.symbol ?? 'UNKNOWN';
  if (!base) return null;

  return {
    label: `${sym}/USDC`,
    baseMint: base,
    quoteMint: USDC_MINT,
    baseDecimals: 6,  // most pump.fun tokens use 6 decimals
    quoteDecimals: 6,
  };
}

/**
 * Fetch recently launched / volatile Solana tokens from DexScreener.
 * Uses two search strategies: Raydium new pairs + boosted/trending.
 */
async function fetchCandidates(): Promise<DsPair[]> {
  const results: DsPair[] = [];
  const seen = new Set<string>();

  // Strategy 1: search for pump.fun graduated tokens on Raydium
  // DexScreener search — finds active Raydium/Solana pairs by keyword
  const pumpSearch = await fetchJson<DsSearchResult>(
    `${DS_BASE}/latest/dex/search?q=pump`,
  );
  if (pumpSearch?.pairs) {
    for (const p of pumpSearch.pairs) {
      if (
        p.chainId === 'solana' &&
        (p.dexId === 'raydium' || p.dexId === 'pumpfun') &&
        p.baseToken?.address &&
        !seen.has(p.baseToken.address)
      ) {
        seen.add(p.baseToken.address);
        results.push(p);
      }
    }
  }

  // Strategy 2: token profiles endpoint — newest token listings across all chains
  const profiles = await fetchJson<DsPair[]>(
    `${DS_BASE}/token-profiles/latest/v1`,
  );
  // Note: this endpoint returns a flat array of token profile objects, not pair objects
  // We use it as a source of mint addresses, then look up their pairs separately
  if (Array.isArray(profiles)) {
    const solanaMints = (profiles as Array<{ tokenAddress?: string; chainId?: string }>)
      .filter((t) => t.chainId === 'solana' && t.tokenAddress)
      .map((t) => t.tokenAddress!)
      .filter((addr) => !seen.has(addr))
      .slice(0, 20);

    if (solanaMints.length > 0) {
      // Batch lookup by token address
      const chunkSize = 10;
      for (let i = 0; i < solanaMints.length; i += chunkSize) {
        const chunk = solanaMints.slice(i, i + chunkSize);
        const tokenData = await fetchJson<{ pairs?: DsPair[] }>(
          `${DS_BASE}/latest/dex/tokens/${chunk.join(',')}`,
        );
        if (tokenData?.pairs) {
          for (const p of tokenData.pairs) {
            if (
              p.chainId === 'solana' &&
              p.baseToken?.address &&
              !seen.has(p.baseToken.address)
            ) {
              seen.add(p.baseToken.address);
              results.push(p);
            }
          }
        }
      }
    }
  }

  return results;
}

export class PumpTokenRegistry {
  private cachedPairs: CrossDexScanPair[] = [];
  private lastRefresh = 0;
  private refreshTimer: ReturnType<typeof setInterval> | null = null;

  /** Start background refresh timer. */
  start(): void {
    if (this.refreshTimer) return;
    // Initial load
    void this.refresh();
    this.refreshTimer = setInterval(() => {
      void this.refresh();
    }, REFRESH_INTERVAL_MS);
  }

  stop(): void {
    if (this.refreshTimer) {
      clearInterval(this.refreshTimer);
      this.refreshTimer = null;
    }
  }

  /** Get current cached pairs, sorted by arb potential. */
  get pairs(): CrossDexScanPair[] {
    return this.cachedPairs;
  }

  /** How old the cache is in seconds. */
  get cacheAgeSeconds(): number {
    return (Date.now() - this.lastRefresh) / 1_000;
  }

  /** Force a refresh. Called automatically by start(). */
  async refresh(): Promise<void> {
    try {
      const candidates = await fetchCandidates();

      // Score and filter
      const scored = candidates
        .map((p) => ({ pair: p, score: scorePair(p) }))
        .filter((s) => s.score > 0)
        .sort((a, b) => b.score - a.score)
        .slice(0, MAX_TRACKED);

      const pairs = scored
        .map((s) => toCrossDexPair(s.pair))
        .filter((p): p is CrossDexScanPair => p !== null);

      this.cachedPairs = pairs;
      this.lastRefresh = Date.now();
    } catch {
      // Keep stale cache on error
    }
  }
}
