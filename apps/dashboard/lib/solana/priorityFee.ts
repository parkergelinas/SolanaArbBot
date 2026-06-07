/**
 * Dynamic priority fee estimation.
 * Samples recent Solana prioritization fees and returns the 75th percentile,
 * clamped to a sane range so we never over-pay on a quiet slot.
 */

import type { Connection } from '@solana/web3.js';

/** Fallback fee when the RPC returns no data or an error (50k micro-lamports = ~0.000026 SOL). */
const DEFAULT_FEE_MICRO_LAMPORTS = 50_000;

/** Hard ceiling — avoid paying more than this even on congested networks. */
const MAX_FEE_MICRO_LAMPORTS = 500_000;

/** Hard floor — ensures the tx is included when blocks aren't completely empty. */
const MIN_FEE_MICRO_LAMPORTS = 5_000;

const PERCENTILE = 0.75;

/**
 * Returns a priority fee in **micro-lamports per compute unit** suitable for passing
 * to `ComputeBudgetProgram.setComputeUnitPrice`.
 *
 * Jupiter's `/swap` endpoint accepts this as `prioritizationFeeLamports` (total lamports),
 * so the caller must multiply by the CU budget if needed — or rely on Jupiter's
 * `dynamicComputeUnitLimit: true` flag to handle it automatically.
 */
export async function estimatePriorityFee(connection: Connection): Promise<number> {
  try {
    const fees = await connection.getRecentPrioritizationFees();
    if (!fees.length) return DEFAULT_FEE_MICRO_LAMPORTS;

    const sorted = fees
      .map((f) => f.prioritizationFee)
      .sort((a, b) => a - b);

    const idx = Math.min(Math.floor(sorted.length * PERCENTILE), sorted.length - 1);
    const raw = sorted[idx] ?? DEFAULT_FEE_MICRO_LAMPORTS;
    return Math.max(MIN_FEE_MICRO_LAMPORTS, Math.min(MAX_FEE_MICRO_LAMPORTS, raw));
  } catch {
    return DEFAULT_FEE_MICRO_LAMPORTS;
  }
}
