import { Connection, PublicKey } from '@solana/web3.js';
import { PUMP_PROGRAM_ID } from './constants.js';
import {
  decodeBondingCurveAccount,
  type BondingCurveState,
} from './bonding-curve.js';

// SEC-4: lightweight base58 format check before passing untrusted strings to PublicKey.
// Catches invalid mints from WebSocket messages or external APIs without a full RPC round-trip.
function isValidBase58Pubkey(address: string): boolean {
  return /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(address);
}

/** Derive bonding curve PDA for a mint. Seeds: ["bonding-curve", mint]. */
export function bondingCurvePda(mint: string): PublicKey {
  if (!isValidBase58Pubkey(mint)) {
    throw new Error(`bondingCurvePda: invalid mint address "${mint}"`);
  }
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from('bonding-curve'), new PublicKey(mint).toBuffer()],
    new PublicKey(PUMP_PROGRAM_ID),
  );
  return pda;
}

/** Fetch bonding curve state from RPC. Returns null if account missing or graduated away. */
export async function fetchBondingCurve(
  connection: Connection,
  mint: string,
): Promise<BondingCurveState | null> {
  try {
    const pda = bondingCurvePda(mint);
    const info = await connection.getAccountInfo(pda, 'confirmed');
    if (!info?.data) return null;
    return decodeBondingCurveAccount(mint, Buffer.from(info.data));
  } catch {
    return null;
  }
}

/** Batch-fetch bonding curves for multiple mints. Silently skips invalid mint addresses. */
export async function fetchBondingCurves(
  connection: Connection,
  mints: string[],
): Promise<Map<string, BondingCurveState>> {
  const out = new Map<string, BondingCurveState>();
  // Filter invalid mints up front so one bad address can't abort the entire batch.
  const validMints = mints.filter(isValidBase58Pubkey);
  const pdas = validMints.map((m) => ({ mint: m, pda: bondingCurvePda(m) }));

  const infos = await connection.getMultipleAccountsInfo(
    pdas.map((p) => p.pda),
    'confirmed',
  );

  for (let i = 0; i < pdas.length; i++) {
    const info = infos[i];
    const { mint } = pdas[i]!; // pdas is built from validMints, so mint is always valid here
    if (!info?.data) continue;
    const state = decodeBondingCurveAccount(mint, Buffer.from(info.data));
    if (state) out.set(mint, state);
  }

  return out;
}
