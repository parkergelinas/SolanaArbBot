import type { ScoringInput, ScoringResult } from './types.js';
/**
 * Score round-trip arb in USD only — never mix atomic UI amounts with USD fees.
 *
 * Flow: input token → intermediate → back to input token.
 * All leg notionals are converted via `MarketState.pricesUsd` before fee subtraction.
 */
export declare function scoreRoundTripUsd(input: ScoringInput): ScoringResult;
/** Estimate execution fees in USD for scoring. */
export declare function estimateFeesUsd(opts: {
    solPriceUsd: number;
    priorityFeeLamports?: number;
    jitoTipLamports?: number;
    slippageBps?: number;
    notionalUsd?: number;
}): import('./types.js').FeeBreakdownUsd;
