import { parsePriceImpactPct, routeHopCount } from '../jupiter/client.js';
import { passesMarketFilter } from '../market/state.js';
export const DEFAULT_FRESHNESS = {
    maxQuoteAgeMs: 800,
    maxForwardReverseDeltaMs: 400,
    maxTimeTakenMs: 500,
    maxHops: 4,
    minSpreadBpsToIgnoreStale: 25,
};
export function evaluateQuoteFreshness(forwardCapturedMs, reverseCapturedMs, forward, reverse, nowMs, cfg = DEFAULT_FRESHNESS) {
    const reasons = [];
    const forwardAge = nowMs - forwardCapturedMs;
    const reverseAge = nowMs - reverseCapturedMs;
    const delta = Math.abs(reverseCapturedMs - forwardCapturedMs);
    if (forwardAge > cfg.maxQuoteAgeMs)
        reasons.push('forward_stale');
    if (reverseAge > cfg.maxQuoteAgeMs)
        reasons.push('reverse_stale');
    if (delta > cfg.maxForwardReverseDeltaMs)
        reasons.push('capture_skew');
    const fwdTaken = forward.timeTaken ?? 0;
    const revTaken = reverse.timeTaken ?? 0;
    if (fwdTaken > cfg.maxTimeTakenMs)
        reasons.push('forward_slow');
    if (revTaken > cfg.maxTimeTakenMs)
        reasons.push('reverse_slow');
    const hops = routeHopCount(forward) + routeHopCount(reverse);
    if (hops > cfg.maxHops * 2)
        reasons.push('route_complex');
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
/** Score executable route quality (hops, impact, token quality, size vs liquidity). */
export function scoreRouteQuality(input) {
    const details = [];
    const hops = routeHopCount(input.forward) + routeHopCount(input.reverse);
    const impact = parsePriceImpactPct(input.forward.priceImpactPct) +
        parsePriceImpactPct(input.reverse.priceImpactPct);
    const baseOk = passesMarketFilter(input.inputMint, input.state);
    const quoteOk = passesMarketFilter(input.outputMint, input.state);
    const tokenQualityOk = baseOk && quoteOk;
    if (!tokenQualityOk)
        details.push('token_quality_fail');
    const liq = input.state.quality[input.inputMint]?.liquidityUsd ?? 0;
    const sizeRatio = liq > 0 ? input.tradeSizeUsd / liq : 1;
    if (sizeRatio > 0.05)
        details.push('size_vs_liquidity_high');
    const diversity = new Set();
    for (const q of [input.forward, input.reverse]) {
        for (const step of q.routePlan ?? []) {
            if (step.swapInfo?.label)
                diversity.add(step.swapInfo.label);
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
