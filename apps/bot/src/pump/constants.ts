/** Pump.fun on-chain program IDs and global constants (from pump-public-docs). */

export const PUMP_PROGRAM_ID = '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P';
export const PUMP_SWAP_PROGRAM_ID = 'pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA';
export const PUMP_GLOBAL_ACCOUNT = '4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf';

/** Initial bonding curve parameters from Global account. */
export const INITIAL_VIRTUAL_TOKEN_RESERVES = 1_073_000_000_000_000n;
export const INITIAL_VIRTUAL_SOL_RESERVES = 30_000_000_000n;
export const INITIAL_REAL_TOKEN_RESERVES = 793_100_000_000_000n;
export const TOKEN_TOTAL_SUPPLY = 1_000_000_000_000_000n;
export const PUMP_FEE_BPS = 100n;

/** Graduation threshold — real_token_reserves hits 0. */
export const GRADUATION_SOL_TARGET_LAMPORTS = 85_000_000_000n;
