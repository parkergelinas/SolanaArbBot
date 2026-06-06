export function computeAnalytics(events) {
    const scans = events.filter((e) => e.type === 'scan').length;
    const decisions = events.filter((e) => e.type === 'decision');
    const executions = events.filter((e) => e.type === 'execution');
    const rejections = events.filter((e) => e.type === 'rejection');
    const expected = executions
        .map((e) => e.expectedProfitUsd ?? 0)
        .filter((n) => Number.isFinite(n));
    const realized = executions
        .map((e) => e.realizedProfitUsd ?? 0)
        .filter((n) => Number.isFinite(n));
    const drifts = executions.map((e) => (e.realizedProfitUsd ?? 0) - (e.expectedProfitUsd ?? 0));
    const routeWins = {};
    for (const e of executions) {
        const key = e.pairLabel ?? 'unknown';
        routeWins[key] ??= { win: 0, total: 0 };
        routeWins[key].total += 1;
        if ((e.realizedProfitUsd ?? 0) > 0)
            routeWins[key].win += 1;
    }
    const routeWinRates = {};
    for (const [k, v] of Object.entries(routeWins)) {
        routeWinRates[k] = v.total > 0 ? v.win / v.total : 0;
    }
    const rejectionReasons = {};
    for (const e of [...rejections, ...decisions.filter((d) => d.rejectionReason)]) {
        const r = e.rejectionReason ?? 'unknown';
        rejectionReasons[r] = (rejectionReasons[r] ?? 0) + 1;
    }
    const actionable = decisions.filter((d) => !d.rejectionReason).length;
    return {
        opportunityHitRate: scans > 0 ? actionable / scans : 0,
        avgExpectedProfitUsd: avg(expected),
        avgRealizedProfitUsd: avg(realized),
        avgQuoteExecutionDriftUsd: avg(drifts),
        routeWinRates,
        rejectionReasons,
        totalScans: scans,
        totalExecutions: executions.length,
    };
}
function avg(nums) {
    if (nums.length === 0)
        return 0;
    return nums.reduce((a, b) => a + b, 0) / nums.length;
}
