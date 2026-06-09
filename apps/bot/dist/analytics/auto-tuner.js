/**
 * Auto-tuner — analyses spread data accumulated during a paper session and
 * produces concrete parameter recommendations for each capital stage.
 *
 * This is intentionally read-only: it never applies changes automatically.
 * Recommendations are surfaced via the /analytics and /backtest endpoints.
 */
import { runBacktest } from '../backtest/engine.js';
const TX_COST_USD = 0.065;
/** Stage definitions — mirrors CapitalTracker stages. */
const STAGES = [
    { name: 'stage1:bootstrap', tradeSizeSol: 0.5, currentSpread: 100, currentProfit: 0.01 },
    { name: 'stage2:growing', tradeSizeSol: 2.0, currentSpread: 60, currentProfit: 0.03 },
    { name: 'stage3:established', tradeSizeSol: 8.0, currentSpread: 35, currentProfit: 0.08 },
    { name: 'stage4:full', tradeSizeSol: 20.0, currentSpread: 25, currentProfit: 0.15 },
];
export function runAutoTuner(events, solPriceUsd = 150) {
    const uptimeSec = (() => {
        const timestamps = events.map((e) => e.timestamp).filter(Boolean);
        if (timestamps.length < 2)
            return 3600;
        return ((Math.max(...timestamps) - Math.min(...timestamps)) / 1000) || 3600;
    })();
    const hoursOfData = uptimeSec / 3600;
    const stageRecommendations = [];
    for (const stage of STAGES) {
        const notionalUsd = stage.tradeSizeSol * solPriceUsd;
        const breakEvenBps = Math.ceil((TX_COST_USD / notionalUsd) * 10_000);
        const backtest = runBacktest(events, stage.tradeSizeSol, solPriceUsd, hoursOfData);
        const best = backtest.best;
        const recommendedSpread = best && best.triggeredTrades > 0
            ? best.config.minSpreadBps
            : Math.max(stage.currentSpread, breakEvenBps + 5);
        const recommendedProfit = best && best.triggeredTrades > 0
            ? best.config.minProfitUsd
            : stage.currentProfit;
        const tradesPerHour = best && hoursOfData > 0
            ? best.triggeredTrades / hoursOfData
            : 0;
        const hourlyProfit = best
            ? best.estimatedTotalProfitUsd / Math.max(hoursOfData, 0.1)
            : 0;
        const viable = tradesPerHour >= 1 && hourlyProfit > 0;
        let viabilityNote;
        if (backtest.observations.length === 0) {
            viabilityNote = 'No spread data yet — run paper session for ≥30 min.';
        }
        else if (backtest.observations.every((o) => o.spreadBps < breakEvenBps)) {
            viabilityNote = `Core pair spreads (max ${Math.max(...backtest.observations.map((o) => o.spreadBps))} bps) `
                + `are below break-even (${breakEvenBps} bps). Needs pump pairs or higher capital.`;
        }
        else if (!viable) {
            viabilityNote = `Too few opportunities at this capital level (<1/hr). `
                + `Viable at Stage ${STAGES.indexOf(stage) + 2}+ with larger notional.`;
        }
        else {
            viabilityNote = `✓ Viable — ${tradesPerHour.toFixed(1)} trades/hr, $${hourlyProfit.toFixed(4)}/hr estimated.`;
        }
        stageRecommendations.push({
            stage: stage.name,
            currentMinSpreadBps: stage.currentSpread,
            recommendedMinSpreadBps: recommendedSpread,
            currentMinProfitUsd: stage.currentProfit,
            recommendedMinProfitUsd: recommendedProfit,
            breakEvenBps,
            estimatedTradesPerHour: tradesPerHour,
            estimatedHourlyProfitUsd: hourlyProfit,
            viable,
            viabilityNote,
        });
    }
    // Top-level suggestion
    const firstViable = stageRecommendations.find((r) => r.viable);
    const allObs = runBacktest(events, 0.5, solPriceUsd, hoursOfData).observations;
    const needsPumpData = allObs.length < 10 || allObs.every((o) => o.spreadBps < 15);
    let topSuggestion;
    if (firstViable) {
        topSuggestion = `${firstViable.stage} is the first viable stage at ${firstViable.estimatedTradesPerHour.toFixed(1)} trades/hr.`;
    }
    else if (needsPumpData) {
        topSuggestion = 'Strategy needs pump.fun token data. Core pairs have no edge at Stage 1–2. '
            + 'The DexScreener pump registry will surface opportunities as it accumulates data. '
            + 'Consider running the paper session for 2–4 hours before evaluating.';
    }
    else {
        topSuggestion = 'Based on current spread data, Stage 3 (10+ SOL) is the minimum viable capital level. '
            + 'Strategy is designed to compound from Stage 1 to Stage 3 using pump pair opportunities.';
    }
    return {
        generatedAt: Date.now(),
        solPriceUsd,
        hoursOfData: parseFloat(hoursOfData.toFixed(2)),
        totalObservations: allObs.length,
        stageRecommendations,
        topSuggestion,
        needsPumpData,
    };
}
