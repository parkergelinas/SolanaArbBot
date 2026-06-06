import type { JupiterTokenMeta, QuotePairSnapshot } from '../jupiter/types.js';
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
