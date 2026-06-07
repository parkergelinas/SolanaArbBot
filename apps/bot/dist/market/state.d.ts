import type { JupiterTokenMeta, QuotePairSnapshot, QuoteSnapshot } from '../jupiter/types.js';
import type { PumpEdgeSignal } from '../pump/edge-scorer.js';
import type { CrossDexQuote } from '../strategy/cross-dex-scanner.js';
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
    /** Per-pair round-trip quotes keyed by pair label. */
    multiQuotes?: Record<string, QuotePairSnapshot>;
    /** Per-pair route divergence keyed by pair label. */
    multiDivergences?: Record<string, RouteDivergenceSnapshot>;
    /** Pump.fun edge signals ranked by score. */
    pumpEdges?: PumpEdgeSignal[];
    /** Cross-DEX (Raydium vs Orca) spread opportunities, sorted by spreadBps desc. */
    crossDexOpportunities?: CrossDexQuote[];
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
export declare const DEFAULT_MARKET_FILTER: MarketFilterConfig;
export declare function passesMarketFilter(mint: string, state: MarketState, cfg?: MarketFilterConfig): boolean;
export declare function tokenQualityFromMeta(meta: JupiterTokenMeta, priceRecencyMs: number, nowMs: number): TokenQuality;
