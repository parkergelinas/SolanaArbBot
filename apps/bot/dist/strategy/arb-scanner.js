import { uiToAtomic } from '../jupiter/client.js';
import { SOL_MINT, USDC_MINT } from '../config/env.js';
export const DEFAULT_SCAN_PAIRS = [
    {
        label: 'SOL/USDC',
        baseMint: SOL_MINT,
        quoteMint: USDC_MINT,
        baseDecimals: 9,
        quoteDecimals: 6,
    },
];
/**
 * Capture forward + reverse Jupiter Swap v1 quotes for round-trip arb analysis.
 */
export async function scanRoundTripQuotes(client, pair, tradeAmountUi, opts = {}) {
    const slippageBps = opts.slippageBps ?? 50;
    const amountAtomic = uiToAtomic(tradeAmountUi, pair.baseDecimals);
    const forwardCaptured = Date.now();
    const forward = await client.getQuote({
        inputMint: pair.baseMint,
        outputMint: pair.quoteMint,
        amount: amountAtomic.toString(),
        slippageBps,
        restrictIntermediateTokens: opts.restrictIntermediateTokens ?? false,
    });
    const reverseCaptured = Date.now();
    const reverse = await client.getQuote({
        inputMint: pair.quoteMint,
        outputMint: pair.baseMint,
        amount: forward.outAmount,
        slippageBps,
        restrictIntermediateTokens: opts.restrictIntermediateTokens ?? false,
    });
    return {
        pairLabel: pair.label,
        forward: {
            request: {
                inputMint: pair.baseMint,
                outputMint: pair.quoteMint,
                amount: amountAtomic.toString(),
                slippageBps,
            },
            response: forward,
            capturedAtMs: forwardCaptured,
        },
        reverse: {
            request: {
                inputMint: pair.quoteMint,
                outputMint: pair.baseMint,
                amount: forward.outAmount,
                slippageBps,
            },
            response: reverse,
            capturedAtMs: reverseCaptured,
        },
    };
}
