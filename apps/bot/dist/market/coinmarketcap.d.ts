/** CoinMarketCap Pro API — top listings + Solana contract resolution. */
export interface CmcListing {
    id: number;
    name: string;
    symbol: string;
    slug: string;
    cmcRank: number;
    priceUsd: number;
    marketCapUsd: number;
    volume24hUsd: number;
}
export interface CmcSolanaToken {
    cmcId: number;
    symbol: string;
    name: string;
    mint: string;
    cmcRank: number;
    priceUsd: number;
    marketCapUsd: number;
}
export declare class CoinMarketCapClient {
    private readonly apiKey;
    constructor(apiKey: string);
    private headers;
    /** Fetch top N cryptocurrencies by market cap. */
    fetchTopListings(limit?: number): Promise<CmcListing[]>;
    /**
     * Resolve Solana mint addresses for CMC coin IDs via `/v2/cryptocurrency/info`.
     * Picks the first Solana-platform contract per coin.
     */
    resolveSolanaMints(listings: CmcListing[]): Promise<CmcSolanaToken[]>;
}
