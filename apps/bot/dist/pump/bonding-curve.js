import { INITIAL_REAL_TOKEN_RESERVES, INITIAL_VIRTUAL_SOL_RESERVES, INITIAL_VIRTUAL_TOKEN_RESERVES, PUMP_FEE_BPS, TOKEN_TOTAL_SUPPLY, } from './constants.js';
/** Graduation progress 0–100. */
export function graduationProgressPct(state) {
    if (state.complete)
        return 100;
    const sold = INITIAL_REAL_TOKEN_RESERVES - state.realTokenReserves;
    return Number((sold * 10000n) / INITIAL_REAL_TOKEN_RESERVES) / 100;
}
/** Spot price in SOL per UI token (6 decimals). */
export function spotPriceSol(state) {
    const vSol = Number(state.virtualSolReserves) / 1e9;
    const vTok = Number(state.virtualTokenReserves) / 1e6;
    if (vTok <= 0)
        return 0;
    return vSol / vTok;
}
/** Spot price in USD given SOL/USD. */
export function spotPriceUsd(state, solPriceUsd) {
    return spotPriceSol(state) * solPriceUsd;
}
/**
 * Uniswap V2 constant-product buy: SOL in → tokens out.
 * Applies 1% protocol fee on input (pump fee_basis_points = 100).
 */
export function buyTokensForSol(state, solLamports) {
    if (state.complete || solLamports <= 0n)
        return 0n;
    const fee = (solLamports * PUMP_FEE_BPS) / 10000n;
    const netSol = solLamports - fee;
    const k = state.virtualSolReserves * state.virtualTokenReserves;
    const newVirtualSol = state.virtualSolReserves + netSol;
    const newVirtualToken = k / newVirtualSol;
    const tokensOut = state.virtualTokenReserves - newVirtualToken;
    const capped = tokensOut > state.realTokenReserves ? state.realTokenReserves : tokensOut;
    return capped > 0n ? capped : 0n;
}
/** Sell tokens → SOL out (after fee). */
export function sellTokensForSol(state, tokenAmount) {
    if (state.complete || tokenAmount <= 0n)
        return 0n;
    const k = state.virtualSolReserves * state.virtualTokenReserves;
    const newVirtualToken = state.virtualTokenReserves + tokenAmount;
    const newVirtualSol = k / newVirtualToken;
    const grossSol = state.virtualSolReserves - newVirtualSol;
    const fee = (grossSol * PUMP_FEE_BPS) / 10000n;
    const netSol = grossSol - fee;
    return netSol > state.realSolReserves ? state.realSolReserves : netSol;
}
/** Price impact in bps for a buy of `solLamports`. */
export function buyPriceImpactBps(state, solLamports) {
    if (solLamports <= 0n)
        return 0;
    const spot = spotPriceSol(state);
    if (spot <= 0)
        return 0;
    const tokensOut = buyTokensForSol(state, solLamports);
    if (tokensOut <= 0n)
        return 10_000;
    const effectivePrice = Number(solLamports) / 1e9 / (Number(tokensOut) / 1e6);
    const impact = ((effectivePrice - spot) / spot) * 10_000;
    return Math.max(0, Math.round(impact));
}
/** Whether curve is in the "sweet spot" for early entry (low SOL raised, low impact). */
export function isEarlyCurveEntry(state) {
    const solRaised = Number(state.realSolReserves) / 1e9;
    const progress = graduationProgressPct(state);
    return !state.complete && solRaised < 15 && progress < 30;
}
/** Near-graduation window where migration arb is possible. */
export function isNearGraduation(state, minPct = 80, maxPct = 99) {
    const progress = graduationProgressPct(state);
    return !state.complete && progress >= minPct && progress <= maxPct;
}
/** Market cap in SOL (virtual reserves proxy). */
export function marketCapSol(state) {
    const price = spotPriceSol(state);
    const supply = Number(TOKEN_TOTAL_SUPPLY) / 1e6;
    return price * supply;
}
/** Decode raw bonding curve account data (Anchor layout, 8-byte discriminator). */
export function decodeBondingCurveAccount(mint, data) {
    if (data.length < 49)
        return null;
    const readU64 = (offset) => {
        return data.readBigUInt64LE(offset);
    };
    return {
        mint,
        virtualTokenReserves: readU64(8),
        virtualSolReserves: readU64(16),
        realTokenReserves: readU64(24),
        realSolReserves: readU64(32),
        tokenTotalSupply: readU64(40),
        complete: data[48] === 1,
    };
}
/** Simulate a fresh bonding curve at creation (for launch scoring). */
export function freshBondingCurve(mint) {
    return {
        mint,
        virtualTokenReserves: INITIAL_VIRTUAL_TOKEN_RESERVES,
        virtualSolReserves: INITIAL_VIRTUAL_SOL_RESERVES,
        realTokenReserves: INITIAL_REAL_TOKEN_RESERVES,
        realSolReserves: 0n,
        tokenTotalSupply: TOKEN_TOTAL_SUPPLY,
        complete: false,
    };
}
