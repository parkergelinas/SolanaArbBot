/**
 * DexScreener price client — free API, no auth, generous rate limits.
 * Used as a fallback when Jupiter's price API is rate-limited.
 *
 * API quirk: multi-token queries return a flat DsPair[], single-token returns
 * { pairs: DsPair[] }. This client normalises both shapes.
 */
/**
 * Fetch USD prices for Solana token mints from DexScreener.
 * Returns a partial map — mints with no liquid pool are omitted.
 */
export declare function fetchDexScreenerPrices(mints: string[]): Promise<Record<string, number>>;
