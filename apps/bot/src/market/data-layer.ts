import { JupiterClient } from '../jupiter/client.js';
import type { JupiterTokenMeta } from '../jupiter/types.js';
import type { BotEnv } from '../config/env.js';
import {
  DEFAULT_MARKET_FILTER,
  type MarketFilterConfig,
  type MarketState,
  tokenQualityFromMeta,
} from './state.js';

export interface UniverseOptions {
  filter?: MarketFilterConfig;
  /** Extra mints always included (e.g. SOL, USDC). */
  alwaysInclude?: string[];
  tokenLimit?: number;
}

/** Build curated token universe from Jupiter Tokens API v2. */
export class TokenUniverse {
  private cache: JupiterTokenMeta[] = [];
  private loadedAtMs = 0;
  private readonly ttlMs: number;

  constructor(
    private readonly client: JupiterClient,
    ttlMs = 300_000,
  ) {
    this.ttlMs = ttlMs;
  }

  async refresh(force = false): Promise<JupiterTokenMeta[]> {
    const now = Date.now();
    if (!force && this.cache.length > 0 && now - this.loadedAtMs < this.ttlMs) {
      return this.cache;
    }
    this.cache = await this.client.fetchVerifiedTokens(5000);
    this.loadedAtMs = now;
    return this.cache;
  }

  async curatedMints(opts: UniverseOptions = {}): Promise<string[]> {
    const filter = opts.filter ?? DEFAULT_MARKET_FILTER;
    const always = new Set(opts.alwaysInclude ?? []);
    const tokens = await this.refresh();
    const now = Date.now();
    const out: string[] = [];

    for (const t of tokens) {
      if (always.has(t.id)) {
        out.push(t.id);
        continue;
      }
      const q = tokenQualityFromMeta(t, 0, now);
      if (filter.requireVerified && !q.verified) continue;
      if (filter.requireStrict && !q.strict) continue;
      if (q.organicScore < filter.minOrganicScore) continue;
      if (q.liquidityUsd < filter.minLiquidityUsd) continue;
      const maxAgeMs = filter.maxListingAgeDays * 86_400_000;
      if (q.listingAgeMs > maxAgeMs) continue;
      out.push(t.id);
      if (opts.tokenLimit && out.length >= opts.tokenLimit) break;
    }

    for (const m of always) {
      if (!out.includes(m)) out.unshift(m);
    }
    return out;
  }
}

/** Aggregates Jupiter Price v3 + Tokens v2 into a single `MarketState`. */
export class MarketDataLayer {
  constructor(
    private readonly client: JupiterClient,
    private readonly universe: TokenUniverse,
  ) {}

  async buildState(
    mints: string[],
    opts: UniverseOptions = {},
  ): Promise<MarketState> {
    const now = Date.now();
    const universe = await this.universe.curatedMints(opts);
    const priceMints = [...new Set([...mints, ...universe.slice(0, 50)])];
    const [pricesRaw, tokens] = await Promise.all([
      this.client.getPrices(priceMints).catch(() => ({}) as import('../jupiter/types.js').PriceV3Response),
      this.universe.refresh(),
    ]);

    const tokenById = new Map(tokens.map((t) => [t.id, t]));
    const pricesUsd: Record<string, number> = {};
    const decimals: Record<string, number> = {};
    const quality: MarketState['quality'] = {};

    for (const mint of priceMints) {
      const entry = pricesRaw[mint];
      if (entry?.usdPrice && entry.usdPrice > 0) {
        pricesUsd[mint] = entry.usdPrice;
      }
      const meta = tokenById.get(mint);
      decimals[mint] = meta?.decimals ?? (mint === 'So11111111111111111111111111111111111111112' ? 9 : 6);
      if (meta) {
        quality[mint] = tokenQualityFromMeta(meta, 0, now);
      }
    }

    const solPriceUsd = pricesUsd['So11111111111111111111111111111111111111112'] ?? 0;

    return {
      timestampMs: now,
      pricesUsd,
      decimals,
      quality,
      universe,
      solPriceUsd,
    };
  }
}

export function createMarketStack(env: BotEnv): {
  client: JupiterClient;
  universe: TokenUniverse;
  dataLayer: MarketDataLayer;
} {
  const client = new JupiterClient(env);
  const universe = new TokenUniverse(client);
  const dataLayer = new MarketDataLayer(client, universe);
  return { client, universe, dataLayer };
}
