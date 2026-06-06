import type { SwapQuoteResponse } from '../jupiter/types.js';
import type { MarketState } from '../market/state.js';
export interface FreshnessConfig {
    maxQuoteAgeMs: number;
    maxForwardReverseDeltaMs: number;
    maxTimeTakenMs: number;
    maxHops: number;
    minSpreadBpsToIgnoreStale: number;
}
export declare const DEFAULT_FRESHNESS: FreshnessConfig;
export interface FreshnessResult {
    score: number;
    stale: boolean;
    reasons: string[];
}
export declare function evaluateQuoteFreshness(forwardCapturedMs: number, reverseCapturedMs: number, forward: SwapQuoteResponse, reverse: SwapQuoteResponse, nowMs: number, cfg?: FreshnessConfig): FreshnessResult;
export interface RouteQualityInput {
    forward: SwapQuoteResponse;
    reverse: SwapQuoteResponse;
    state: MarketState;
    inputMint: string;
    outputMint: string;
    tradeSizeUsd: number;
}
export interface RouteQualityResult {
    score: number;
    hops: number;
    priceImpactPct: number;
    tokenQualityOk: boolean;
    details: string[];
}
/** Score executable route quality (hops, impact, token quality, size vs liquidity). */
export declare function scoreRouteQuality(input: RouteQualityInput): RouteQualityResult;
