import { buyPriceImpactBps, buyTokensForSol, graduationProgressPct, isEarlyCurveEntry, isNearGraduation, spotPriceUsd, } from './bonding-curve.js';
export const DEFAULT_PUMP_EDGE_CONFIG = {
    minEdgeBps: 50,
    tradeSol: 0.5,
    minGraduationPct: 80,
    maxGraduationPct: 99,
    maxEarlyImpactBps: 300,
    minCurveJupiterDivergenceBps: 75,
};
/**
 * Score a pump.fun bonding curve for tradeable edge.
 *
 * Edge logic (from pump-public-docs + bonding curve math):
 * 1. Curve vs Jupiter divergence — buy cheap on curve, sell on Jupiter (or reverse)
 * 2. Graduation proximity — tokens 80–99% graduated often misprice vs post-migration AMM
 * 3. Early curve entry — fresh launches with low SOL raised + low price impact
 * 4. Migration arb — near-complete curves where curve price < Jupiter implies buy-before-migrate
 */
export function scorePumpEdge(state, solPriceUsd, jupiterPriceUsd, cfg = DEFAULT_PUMP_EDGE_CONFIG) {
    const graduationPct = graduationProgressPct(state);
    const curvePriceUsd = spotPriceUsd(state, solPriceUsd);
    const solRaised = Number(state.realSolReserves) / 1e9;
    const tradeLamports = BigInt(Math.round(cfg.tradeSol * 1e9));
    const priceImpactBps = buyPriceImpactBps(state, tradeLamports);
    let best = null;
    // 1. Curve vs Jupiter divergence
    if (jupiterPriceUsd && jupiterPriceUsd > 0 && curvePriceUsd > 0) {
        const divergenceBps = Math.round(((jupiterPriceUsd - curvePriceUsd) / curvePriceUsd) * 10_000);
        const absDiv = Math.abs(divergenceBps);
        if (absDiv >= cfg.minCurveJupiterDivergenceBps) {
            const edgeScore = Math.min(100, Math.round(absDiv / 5));
            const signal = {
                mint: state.mint,
                signalType: divergenceBps > 0 ? 'curve_jupiter_divergence' : 'migration_arb',
                edgeScore,
                edgeBps: absDiv,
                curvePriceUsd,
                jupiterPriceUsd,
                graduationPct,
                priceImpactBps,
                solRaised,
                complete: state.complete,
                metadata: {
                    direction: divergenceBps > 0 ? 'buy_curve_sell_jupiter' : 'buy_jupiter_sell_curve',
                    divergenceBps,
                },
            };
            if (best === null || signal.edgeScore > best.edgeScore)
                best = signal;
        }
    }
    // 2. Graduation proximity play
    if (isNearGraduation(state, cfg.minGraduationPct, cfg.maxGraduationPct)) {
        const proximityBonus = Math.round((graduationPct - cfg.minGraduationPct) * 2);
        const edgeBps = 30 + proximityBonus;
        const signal = {
            mint: state.mint,
            signalType: 'graduation_proximity',
            edgeScore: Math.min(90, 40 + proximityBonus),
            edgeBps,
            curvePriceUsd,
            jupiterPriceUsd,
            graduationPct,
            priceImpactBps,
            solRaised,
            complete: state.complete,
            metadata: {
                strategy: 'front_run_migration',
                tokensRemainingPct: 100 - graduationPct,
            },
        };
        if (best === null || signal.edgeScore > best.edgeScore)
            best = signal;
    }
    // 3. Early curve entry — low SOL raised, acceptable impact
    if (isEarlyCurveEntry(state) && priceImpactBps <= cfg.maxEarlyImpactBps) {
        const tokensOut = buyTokensForSol(state, tradeLamports);
        const edgeBps = Math.max(20, 200 - priceImpactBps);
        const signal = {
            mint: state.mint,
            signalType: 'early_curve_entry',
            edgeScore: Math.min(75, 50 + Math.round((cfg.maxEarlyImpactBps - priceImpactBps) / 10)),
            edgeBps,
            curvePriceUsd,
            jupiterPriceUsd,
            graduationPct,
            priceImpactBps,
            solRaised,
            complete: state.complete,
            metadata: {
                tokensOutUi: Number(tokensOut) / 1e6,
                tradeSol: cfg.tradeSol,
            },
        };
        if (best === null || (signal.edgeScore > best.edgeScore && signal.edgeBps >= cfg.minEdgeBps)) {
            best = signal;
        }
    }
    // 4. Fresh launch — just created, minimal SOL raised
    if (solRaised < 1 && graduationPct < 5 && priceImpactBps < 150) {
        const signal = {
            mint: state.mint,
            signalType: 'fresh_launch',
            edgeScore: 60,
            edgeBps: 100,
            curvePriceUsd,
            jupiterPriceUsd,
            graduationPct,
            priceImpactBps,
            solRaised,
            complete: state.complete,
            metadata: { strategy: 'snipe_fresh_launch' },
        };
        if (best === null || signal.edgeScore > best.edgeScore)
            best = signal;
    }
    if (best === null || best.edgeBps < cfg.minEdgeBps)
        return null;
    return best;
}
/** Rank multiple signals by edge score. */
export function rankPumpEdges(signals) {
    return [...signals].sort((a, b) => b.edgeScore - a.edgeScore);
}
