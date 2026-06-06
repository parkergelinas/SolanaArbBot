import { atomicToUi } from '../jupiter/client.js';
import { scanRoundTripQuotes } from './arb-scanner.js';
const ROUTE_CONSTRUCTIONS = [
    { label: 'restricted', restrictIntermediateTokens: true },
    { label: 'unrestricted', restrictIntermediateTokens: false },
];
/**
 * Capture round-trip quotes under multiple route construction policies and
 * measure divergence between the best and worst outcomes.
 */
export async function scanRouteDivergence(client, pair, tradeAmountUi, opts = {}) {
    const constructions = await Promise.all(ROUTE_CONSTRUCTIONS.map(async (cfg) => {
        const roundTrip = await scanRoundTripQuotes(client, pair, tradeAmountUi, {
            ...opts,
            restrictIntermediateTokens: cfg.restrictIntermediateTokens,
        });
        return {
            label: cfg.label,
            restrictIntermediateTokens: cfg.restrictIntermediateTokens,
            forward: roundTrip.forward,
            reverse: roundTrip.reverse,
        };
    }));
    const endUiByLabel = constructions.map((c) => ({
        label: c.label,
        endUi: atomicToUi(c.reverse.response.outAmount, pair.baseDecimals),
    }));
    const sorted = [...endUiByLabel].sort((a, b) => b.endUi - a.endUi);
    const best = sorted[0];
    const worst = sorted[sorted.length - 1];
    const mid = (best.endUi + worst.endUi) / 2;
    const divergenceBps = mid > 0 ? Math.round(((best.endUi - worst.endUi) / mid) * 10_000) : 0;
    return {
        pairLabel: pair.label,
        constructions,
        bestConstructionLabel: best.label,
        divergenceBps,
    };
}
