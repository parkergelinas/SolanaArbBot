export const DEFAULT_RISK_LIMITS = {
    maxSessionLossUsd: 50,
    maxInventoryUsdPerAsset: 500,
    maxConsecutiveQuoteFailures: 5,
    maxRouteFamilyFailures: 3,
    cooldownAfterPoorExecutionMs: 60_000,
    minSpreadBpsAfterCompression: 12,
};
export function checkStrategyRisk(decision, state, limits = DEFAULT_RISK_LIMITS, nowMs = Date.now()) {
    if (nowMs < state.cooldownUntilMs) {
        return { verdict: 'reject', reason: 'cooldown_active' };
    }
    if (state.sessionLossUsd >= limits.maxSessionLossUsd) {
        return { verdict: 'halt', reason: 'max_session_loss' };
    }
    if (state.consecutiveQuoteFailures >= limits.maxConsecutiveQuoteFailures) {
        return { verdict: 'halt', reason: 'quote_failure_streak' };
    }
    const family = decision.pairLabel;
    const famFails = state.routeFamilyFailures[family] ?? 0;
    if (famFails >= limits.maxRouteFamilyFailures) {
        return { verdict: 'reject', reason: 'route_family_cooldown' };
    }
    const inv = state.inventoryUsd[decision.inputMint] ?? 0;
    if (inv >= limits.maxInventoryUsdPerAsset) {
        return { verdict: 'reject', reason: 'max_inventory' };
    }
    if (decision.grossSpreadBps < limits.minSpreadBpsAfterCompression &&
        decision.rejectionReason?.includes('stale')) {
        return { verdict: 'reject', reason: 'spread_compression' };
    }
    if (decision.rejectionReason && !decision.rejectionReason.startsWith('below_')) {
        return { verdict: 'reject', reason: decision.rejectionReason };
    }
    return { verdict: 'allow' };
}
export function recordExecutionOutcome(state, decision, realizedUsd, success, limits = DEFAULT_RISK_LIMITS, nowMs = Date.now()) {
    const next = { ...state, routeFamilyFailures: { ...state.routeFamilyFailures } };
    if (!success) {
        next.consecutiveQuoteFailures += 1;
        const fam = decision.pairLabel;
        next.routeFamilyFailures[fam] = (next.routeFamilyFailures[fam] ?? 0) + 1;
        if (next.consecutiveQuoteFailures >= limits.maxConsecutiveQuoteFailures) {
            next.cooldownUntilMs = nowMs + limits.cooldownAfterPoorExecutionMs;
            next.lastHaltReason = 'quote_failures';
        }
        return next;
    }
    next.consecutiveQuoteFailures = 0;
    if (realizedUsd < 0) {
        next.sessionLossUsd += Math.abs(realizedUsd);
    }
    return next;
}
