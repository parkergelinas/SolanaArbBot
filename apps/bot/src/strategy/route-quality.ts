import { parsePriceImpactPct, routeHopCount } from '../jupiter/client.js';
import type { SwapQuoteResponse } from '../jupiter/types.js';
import type { MarketState } from '../market/state.js';
import { passesMarketFilter } from '../market/state.js';

export interface FreshnessConfig {
  maxQuoteAgeMs: number;
  maxForwardReverseDeltaMs: number;
  maxTimeTakenMs: number;
  maxHops: number;
  minSpreadBpsToIgnoreStale: number;
}

export const DEFAULT_FRESHNESS: FreshnessConfig = {
  maxQuoteAgeMs: 800,
  maxForwardReverseDeltaMs: 400,
  maxTimeTakenMs: 500,
  maxHops: 4,
  minSpreadBpsToIgnoreStale: 25,
};

export interface FreshnessResult {
  score: number;
  stale: boolean;
  reasons: string[];
}

export function evaluateQuoteFreshness(
  forwardCapturedMs: number,
  reverseCapturedMs: number,
  forward: SwapQuoteResponse,
  reverse: SwapQuoteResponse,
  nowMs: number,
  cfg: FreshnessConfig = DEFAULT_FRESHNESS,
): FreshnessResult {
  const reasons: string[] = [];
  const forwardAge = nowMs - forwardCapturedMs;
  const reverseAge = nowMs - reverseCapturedMs;
  const delta = Math.abs(reverseCapturedMs - forwardCapturedMs);

  if (forwardAge > cfg.maxQuoteAgeMs) reasons.push('forward_stale');
  if (reverseAge > cfg.maxQuoteAgeMs) reasons.push('reverse_stale');
  if (delta > cfg.maxForwardReverseDeltaMs) reasons.push('capture_skew');

  const fwdTaken = forward.timeTaken ?? 0;
  const revTaken = reverse.timeTaken ?? 0;
  if (fwdTaken > cfg.maxTimeTakenMs) reasons.push('forward_slow');
  if (revTaken > cfg.maxTimeTakenMs) reasons.push('reverse_slow');

  const hops = routeHopCount(forward) + routeHopCount(reverse);
  if (hops > cfg.maxHops * 2) reasons.push('route_complex');

  let score = 1;
  score -= Math.min(0.4, forwardAge / cfg.maxQuoteAgeMs / 2);
  score -= Math.min(0.3, delta / cfg.maxForwardReverseDeltaMs / 2);
  score -= Math.min(0.2, hops / (cfg.maxHops * 4));

  return {
    score: Math.max(0, score),
    stale: reasons.length > 0,
    reasons,
  };
}

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
export function scoreRouteQuality(input: RouteQualityInput): RouteQualityResult {
  const details: string[] = [];
  const hops = routeHopCount(input.forward) + routeHopCount(input.reverse);
  const impact =
    parsePriceImpactPct(input.forward.priceImpactPct) +
    parsePriceImpactPct(input.reverse.priceImpactPct);

  const baseOk = passesMarketFilter(input.inputMint, input.state);
  const quoteOk = passesMarketFilter(input.outputMint, input.state);
  const tokenQualityOk = baseOk && quoteOk;
  if (!tokenQualityOk) details.push('token_quality_fail');

  const liq = input.state.quality[input.inputMint]?.liquidityUsd ?? 0;
  const sizeRatio = liq > 0 ? input.tradeSizeUsd / liq : 1;
  if (sizeRatio > 0.05) details.push('size_vs_liquidity_high');

  const diversity = new Set<string>();
  for (const q of [input.forward, input.reverse]) {
    for (const step of q.routePlan ?? []) {
      if (step.swapInfo?.label) diversity.add(step.swapInfo.label);
    }
  }

  let score = 1;
  score -= Math.min(0.35, hops * 0.08);
  score -= Math.min(0.35, impact / 100);
  score -= tokenQualityOk ? 0 : 0.4;
  score -= Math.min(0.2, sizeRatio * 2);
  score += Math.min(0.1, diversity.size * 0.03);

  return {
    score: Math.max(0, Math.min(1, score)),
    hops,
    priceImpactPct: impact,
    tokenQualityOk,
    details,
  };
}
