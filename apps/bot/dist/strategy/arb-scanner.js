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
const QUOTE_TIMEOUT_MS = 8_000;
function withTimeout(promise, ms, label) {
    return Promise.race([
        promise,
        new Promise((_, reject) => setTimeout(() => reject(new Error(`timeout:${label}`)), ms)),
    ]);
}
/**
 * Capture forward + reverse Jupiter Swap v1 quotes for round-trip arb analysis.
 * Each quote call is wrapped in an 8 s timeout so a stalled RPC cannot freeze
 * the engine scan loop.
 */
export async function scanRoundTripQuotes(client, pair, tradeAmountUi, opts = {}) {
    const slippageBps = opts.slippageBps ?? 50;
    const amountAtomic = uiToAtomic(tradeAmountUi, pair.baseDecimals);
    const forwardCaptured = Date.now();
    const forward = await withTimeout(client.getQuote({
        inputMint: pair.baseMint,
        outputMint: pair.quoteMint,
        amount: amountAtomic.toString(),
        slippageBps,
        restrictIntermediateTokens: opts.restrictIntermediateTokens ?? false,
    }), QUOTE_TIMEOUT_MS, `forward:${pair.label}`);
    const reverseCaptured = Date.now();
    const reverse = await withTimeout(client.getQuote({
        inputMint: pair.quoteMint,
        outputMint: pair.baseMint,
        amount: forward.outAmount,
        slippageBps,
        restrictIntermediateTokens: opts.restrictIntermediateTokens ?? false,
    }), QUOTE_TIMEOUT_MS, `reverse:${pair.label}`);
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
