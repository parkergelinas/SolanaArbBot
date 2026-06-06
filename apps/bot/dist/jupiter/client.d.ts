import type { BotEnv } from '../config/env.js';
import type { JupiterTokenMeta, PriceV3Response, SwapQuoteRequest, SwapQuoteResponse, SwapTransactionRequest, SwapTransactionResponse } from './types.js';
export declare class JupiterClient {
    private readonly env;
    constructor(env: BotEnv);
    private headers;
    buildQuoteUrl(req: SwapQuoteRequest): string;
    getQuote(req: SwapQuoteRequest): Promise<SwapQuoteResponse>;
    getSwapTransaction(req: SwapTransactionRequest): Promise<SwapTransactionResponse>;
    getPrices(mints: string[]): Promise<PriceV3Response>;
    fetchVerifiedTokens(limit?: number): Promise<JupiterTokenMeta[]>;
    fetchStrictTokens(limit?: number): Promise<JupiterTokenMeta[]>;
}
/** Parse atomic string amount to UI float using decimals. */
export declare function atomicToUi(amount: string, decimals: number): number;
export declare function uiToAtomic(amountUi: number, decimals: number): bigint;
export declare function parsePriceImpactPct(raw: string | number | undefined): number;
export declare function routeHopCount(quote: SwapQuoteResponse): number;
