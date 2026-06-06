import { atomicToUi } from '../jupiter/client.js';
/**
 * Score round-trip arb in USD only — never mix atomic UI amounts with USD fees.
 *
 * Flow: input token → intermediate → back to input token.
 * All leg notionals are converted via `MarketState.pricesUsd` before fee subtraction.
 */
export function scoreRoundTripUsd(input) {
    const { pair, state, inputDecimals, feesUsd } = input;
    const inputMint = pair.forward.request.inputMint;
    const priceIn = state.pricesUsd[inputMint] ?? state.solPriceUsd;
    if (priceIn <= 0) {
        return {
            netProfitUsd: 0,
            grossSpreadBps: 0,
            startUsd: 0,
            endUsd: 0,
            rejected: true,
            rejectionReason: 'missing_input_price',
        };
    }
    const startUi = atomicToUi(pair.forward.request.amount, inputDecimals);
    const endUi = atomicToUi(pair.reverse.response.outAmount, inputDecimals);
    const startUsd = startUi * priceIn;
    const endUsd = endUi * priceIn;
    const grossUsd = endUsd - startUsd;
    const netProfitUsd = grossUsd - feesUsd.totalUsd;
    const grossSpreadBps = startUsd > 0 ? Math.round((grossUsd / startUsd) * 10_000) : 0;
    return {
        netProfitUsd,
        grossSpreadBps,
        startUsd,
        endUsd,
        rejected: false,
    };
}
/** Estimate execution fees in USD for scoring. */
export function estimateFeesUsd(opts) {
    const lamportsPerSol = 1_000_000_000;
    const priority = ((opts.priorityFeeLamports ?? 50_000) / lamportsPerSol) * opts.solPriceUsd;
    const jito = ((opts.jitoTipLamports ?? 10_000) / lamportsPerSol) * opts.solPriceUsd;
    const slip = opts.notionalUsd && opts.slippageBps
        ? opts.notionalUsd * (opts.slippageBps / 10_000)
        : 0;
    return {
        priorityFeeUsd: priority,
        jitoTipUsd: jito,
        slippageUsd: slip,
        totalUsd: priority + jito + slip,
    };
}
