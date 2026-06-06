/**
 * Static fallback: CMC top-100 symbols → Solana mint addresses.
 * Used when CMC API key is absent or info endpoint returns no contract.
 * Sources: Jupiter verified list + Solscan token registry.
 */
export interface StaticSolanaToken {
    symbol: string;
    name: string;
    mint: string;
    decimals: number;
    cmcRankHint?: number;
}
/** Well-known Solana-native / wrapped tokens from CMC top 100. */
export declare const STATIC_SOLANA_TOKENS: StaticSolanaToken[];
export declare function staticMintForSymbol(symbol: string): StaticSolanaToken | undefined;
export declare function staticTokenForMint(mint: string): StaticSolanaToken | undefined;
