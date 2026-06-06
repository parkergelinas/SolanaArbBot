/** Parsed bonding curve account state. */
export interface BondingCurveState {
    mint: string;
    virtualTokenReserves: bigint;
    virtualSolReserves: bigint;
    realTokenReserves: bigint;
    realSolReserves: bigint;
    tokenTotalSupply: bigint;
    complete: boolean;
}
/** Graduation progress 0–100. */
export declare function graduationProgressPct(state: BondingCurveState): number;
/** Spot price in SOL per UI token (6 decimals). */
export declare function spotPriceSol(state: BondingCurveState): number;
/** Spot price in USD given SOL/USD. */
export declare function spotPriceUsd(state: BondingCurveState, solPriceUsd: number): number;
/**
 * Uniswap V2 constant-product buy: SOL in → tokens out.
 * Applies 1% protocol fee on input (pump fee_basis_points = 100).
 */
export declare function buyTokensForSol(state: BondingCurveState, solLamports: bigint): bigint;
/** Sell tokens → SOL out (after fee). */
export declare function sellTokensForSol(state: BondingCurveState, tokenAmount: bigint): bigint;
/** Price impact in bps for a buy of `solLamports`. */
export declare function buyPriceImpactBps(state: BondingCurveState, solLamports: bigint): number;
/** Whether curve is in the "sweet spot" for early entry (low SOL raised, low impact). */
export declare function isEarlyCurveEntry(state: BondingCurveState): boolean;
/** Near-graduation window where migration arb is possible. */
export declare function isNearGraduation(state: BondingCurveState, minPct?: number, maxPct?: number): boolean;
/** Market cap in SOL (virtual reserves proxy). */
export declare function marketCapSol(state: BondingCurveState): number;
/** Decode raw bonding curve account data (Anchor layout, 8-byte discriminator). */
export declare function decodeBondingCurveAccount(mint: string, data: Buffer): BondingCurveState | null;
/** Simulate a fresh bonding curve at creation (for launch scoring). */
export declare function freshBondingCurve(mint: string): BondingCurveState;
