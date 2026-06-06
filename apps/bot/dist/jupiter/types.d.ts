/** Jupiter Swap API v1 quote/swap shapes (`api.jup.ag/swap/v1`). */
export interface SwapQuoteRequest {
    inputMint: string;
    outputMint: string;
    amount: string | number;
    slippageBps?: number;
    restrictIntermediateTokens?: boolean;
    swapMode?: 'ExactIn' | 'ExactOut';
}
export interface RoutePlanStep {
    swapInfo?: {
        ammKey?: string;
        label?: string;
        inputMint?: string;
        outputMint?: string;
        inAmount?: string;
        outAmount?: string;
        feeAmount?: string;
        feeMint?: string;
    };
    percent?: number;
}
export interface SwapQuoteResponse {
    inputMint?: string;
    inAmount: string;
    outputMint?: string;
    outAmount: string;
    otherAmountThreshold?: string;
    swapMode?: string;
    slippageBps?: number;
    priceImpactPct?: string | number;
    routePlan?: RoutePlanStep[];
    contextSlot?: number;
    timeTaken?: number;
}
export interface SwapTransactionRequest {
    quoteResponse: SwapQuoteResponse;
    userPublicKey: string;
    wrapAndUnwrapSol?: boolean;
    dynamicComputeUnitLimit?: boolean;
    prioritizationFeeLamports?: number | 'auto';
}
export interface SwapTransactionResponse {
    swapTransaction: string;
    lastValidBlockHeight?: number;
}
export interface JupiterTokenMeta {
    id: string;
    symbol?: string;
    name?: string;
    decimals?: number;
    tags?: string[];
    organicScore?: number;
    liquidity?: number;
    createdAt?: string;
    updatedAt?: string;
}
export interface PriceV3Entry {
    usdPrice?: number;
    priceChange24h?: number;
    blockId?: number;
}
export type PriceV3Response = Record<string, PriceV3Entry>;
export interface QuoteSnapshot {
    request: SwapQuoteRequest;
    response: SwapQuoteResponse;
    capturedAtMs: number;
}
export interface QuotePairSnapshot {
    forward: QuoteSnapshot;
    reverse: QuoteSnapshot;
    pairLabel: string;
}
