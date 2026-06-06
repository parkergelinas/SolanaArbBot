import { PublicKey } from '@solana/web3.js';
import { PUMP_PROGRAM_ID } from './constants.js';
import { decodeBondingCurveAccount, } from './bonding-curve.js';
/** Derive bonding curve PDA for a mint. Seeds: ["bonding-curve", mint]. */
export function bondingCurvePda(mint) {
    const [pda] = PublicKey.findProgramAddressSync([Buffer.from('bonding-curve'), new PublicKey(mint).toBuffer()], new PublicKey(PUMP_PROGRAM_ID));
    return pda;
}
/** Fetch bonding curve state from RPC. Returns null if account missing or graduated away. */
export async function fetchBondingCurve(connection, mint) {
    try {
        const pda = bondingCurvePda(mint);
        const info = await connection.getAccountInfo(pda, 'confirmed');
        if (!info?.data)
            return null;
        return decodeBondingCurveAccount(mint, Buffer.from(info.data));
    }
    catch {
        return null;
    }
}
/** Batch-fetch bonding curves for multiple mints. */
export async function fetchBondingCurves(connection, mints) {
    const out = new Map();
    const pdas = mints.map((m) => ({ mint: m, pda: bondingCurvePda(m) }));
    const infos = await connection.getMultipleAccountsInfo(pdas.map((p) => p.pda), 'confirmed');
    for (let i = 0; i < pdas.length; i++) {
        const info = infos[i];
        const { mint } = pdas[i];
        if (!info?.data)
            continue;
        const state = decodeBondingCurveAccount(mint, Buffer.from(info.data));
        if (state)
            out.set(mint, state);
    }
    return out;
}
