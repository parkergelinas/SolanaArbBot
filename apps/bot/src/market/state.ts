import type { JupiterTokenMeta, QuotePairSnapshot, QuoteSnapshot } from '../jupiter/types.js';

/** One round-trip quote capture under a specific route construction policy. */
export interface RouteConstructionSnapshot {
  label: string;
  restrictIntermediateTokens: boolean;
  forward: QuoteSnapshot;
  reverse: QuoteSnapshot;
}

/** Multiple route constructions for the same pair — used by route-divergence arb. */
export interface RouteDivergenceSnapshot {
  pairLabel: string;
  constructions: RouteConstructionSnapshot[];
  bestConstructionLabel: string;
  /** Spread in bps between best and worst round-trip outcomes. */
  divergenceBps: number;
}

export interface TokenQuality {
  verified: boolean;
  strict: boolean;
  organicScore: number;
  liquidityUsd: number;
  listingAgeMs: number;
  priceRecencyMs: number;
}

export interface MarketState {
  timestampMs: number;
  /** USD price per token unit (UI amount). */
  pricesUsd: Record<string, number>;
  /** Token decimals for UI ↔ atomic conversion. */
  decimals: Record<string, number>;
  /** Quality metadata keyed by mint. */
  quality: Record<string, TokenQuality>;
  /** Curated tradeable universe (mint addresses). */
  universe: string[];
  /** Optional round-trip quote pair from scanner. */
  quotes?: QuotePairSnapshot;
  /** Multi-route construction comparison from route-divergence scanner. */
  routeDivergence?: RouteDivergenceSnapshot;
  solPriceUsd: number;
}

export interface MarketFilterConfig {
  requireVerified: boolean;
  requireStrict: boolean;
  minOrganicScore: number;
  minLiquidityUsd: number;
  maxListingAgeDays: number;
  maxPriceStaleMs: number;
}

export const DEFAULT_MARKET_FILTER: MarketFilterConfig = {
  requireVerified: true,
  requireStrict: false,
  minOrganicScore: 0,
  minLiquidityUsd: 10_000,
  maxListingAgeDays: 365,
  maxPriceStaleMs: 60_000,
};

export function passesMarketFilter(
  mint: string,
  state: MarketState,
  cfg: MarketFilterConfig = DEFAULT_MARKET_FILTER,
): boolean {
  const q = state.quality[mint];
  if (!q) return false;
  if (cfg.requireVerified && !q.verified) return false;
  if (cfg.requireStrict && !q.strict) return false;
  if (q.organicScore < cfg.minOrganicScore) return false;
  if (q.liquidityUsd < cfg.minLiquidityUsd) return false;
  const maxAgeMs = cfg.maxListingAgeDays * 86_400_000;
  if (q.listingAgeMs > maxAgeMs) return false;
  if (q.priceRecencyMs > cfg.maxPriceStaleMs) return false;
  return state.universe.includes(mint);
}

export function tokenQualityFromMeta(
  meta: JupiterTokenMeta,
  priceRecencyMs: number,
  nowMs: number,
): TokenQuality {
  const tags = meta.tags ?? [];
  const created = meta.createdAt ? Date.parse(meta.createdAt) : nowMs;
  return {
    verified: tags.some((t) => t.toLowerCase() === 'verified'),
    strict: tags.some((t) => t.toLowerCase() === 'strict'),
    organicScore: meta.organicScore ?? 0,
    liquidityUsd: meta.liquidity ?? 0,
    listingAgeMs: Math.max(0, nowMs - (Number.isFinite(created) ? created : nowMs)),
    priceRecencyMs,
  };
}
