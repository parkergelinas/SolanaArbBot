import { Connection, PublicKey } from '@solana/web3.js';
import { type BondingCurveState } from './bonding-curve.js';
/** Derive bonding curve PDA for a mint. Seeds: ["bonding-curve", mint]. */
export declare function bondingCurvePda(mint: string): PublicKey;
/** Fetch bonding curve state from RPC. Returns null if account missing or graduated away. */
export declare function fetchBondingCurve(connection: Connection, mint: string): Promise<BondingCurveState | null>;
/** Batch-fetch bonding curves for multiple mints. Silently skips invalid mint addresses. */
export declare function fetchBondingCurves(connection: Connection, mints: string[]): Promise<Map<string, BondingCurveState>>;
