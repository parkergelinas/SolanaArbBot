export const DEFAULT_MARKET_FILTER = {
    requireVerified: true,
    requireStrict: false,
    minOrganicScore: 0,
    minLiquidityUsd: 10_000,
    maxListingAgeDays: 365,
    maxPriceStaleMs: 60_000,
};
export function passesMarketFilter(mint, state, cfg = DEFAULT_MARKET_FILTER) {
    const q = state.quality[mint];
    if (!q)
        return false;
    if (cfg.requireVerified && !q.verified)
        return false;
    if (cfg.requireStrict && !q.strict)
        return false;
    if (q.organicScore < cfg.minOrganicScore)
        return false;
    if (q.liquidityUsd < cfg.minLiquidityUsd)
        return false;
    const maxAgeMs = cfg.maxListingAgeDays * 86_400_000;
    if (q.listingAgeMs > maxAgeMs)
        return false;
    if (q.priceRecencyMs > cfg.maxPriceStaleMs)
        return false;
    return state.universe.includes(mint);
}
export function tokenQualityFromMeta(meta, priceRecencyMs, nowMs) {
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
