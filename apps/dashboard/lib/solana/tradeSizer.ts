/**
 * MEV-aware trade sizer — browser / wallet-adapter edition.
 *
 * Mirrors the bot's sizing/dynamic.ts logic but simplified for the frontend:
 * - No Jito (wallet adapter signs and sends directly; Jito bundles require
 *   server-side submission, which is not available in the browser).
 * - Ceiling is always the no-Jito path: min(0.42 SOL, mev_safe_sol).
 * - Secondary factors (failure rate, liquidity, congestion) are passed
 *   optionally and default to healthy values so the function stays callable
 *   with just (spreadBps, solPriceUsd).
 *
 * The fundamental equation:
 *
 *   mev_safe_sol = MEV_THRESHOLD_USD / (spread_bps / 10_000 × sol_price_usd)
 *
 * This is the largest trade where sandwiching produces less than the attacker's
 * tip cost (~$0.50 on current Solana mempool economics).
 */

/** $USD value above which a sandwich attack on this trade becomes profitable. */
const MEV_THRESHOLD_USD = 0.50;

/** Hard ceiling without Jito protection. */
const NO_JITO_MAX_SOL = 0.42;

/** Minimum trade size — below this the spread rarely survives network latency. */
const MIN_SOL = 0.10;

export interface FrontendSizerOptions {
  /** Recent failure fraction 0–1 (e.g. from paper store's error history). Default 0. */
  recentFailureRate?: number;
  /** Pool liquidity in USD. Default 500_000 (assume deep SOL/USDC pool). */
  liquidityUsd?: number;
  /** Estimated priority fee in micro-lamports. Default 50_000. */
  priorityFeeMicroLamports?: number;
}

export interface FrontendSizeResult {
  /** Optimal trade size in SOL (rounded to 2 dp). */
  solAmount: number;
  /** MEV-safe size as computed from the threshold equation. */
  mevSafeSol: number;
  /** Binding ceiling label for display. */
  binding: 'mev_threshold' | 'no_jito_cap' | 'min_clamp';
  /** One-line human-readable explanation. */
  label: string;
}

/**
 * Compute the optimal trade size for a single arb opportunity when executing
 * from a browser wallet (no Jito protection).
 *
 * @param spreadBps  Gross spread detected (basis points)
 * @param solPriceUsd  Current SOL/USD price
 * @param opts  Optional secondary factor overrides
 */
export function computeFrontendTradeSize(
  spreadBps: number,
  solPriceUsd: number,
  opts: FrontendSizerOptions = {},
): FrontendSizeResult {
  const failureRate = opts.recentFailureRate ?? 0;
  const liquidityUsd = opts.liquidityUsd ?? 500_000;
  const feeMicroLamports = opts.priorityFeeMicroLamports ?? 50_000;

  // ── 1. MEV sandwich ceiling ──────────────────────────────────────────────
  const mevSafeSol =
    solPriceUsd > 0 && spreadBps > 0
      ? MEV_THRESHOLD_USD / ((spreadBps / 10_000) * solPriceUsd)
      : NO_JITO_MAX_SOL;

  // Effective ceiling: whichever is tighter
  const ceilingSol = Math.min(NO_JITO_MAX_SOL, mevSafeSol);
  const binding: FrontendSizeResult['binding'] =
    mevSafeSol < NO_JITO_MAX_SOL ? 'mev_threshold' : 'no_jito_cap';

  // ── 2. Secondary factors (downward only) ─────────────────────────────────

  // Spread confidence: wider spread → opportunity more likely to survive latency
  let spreadFactor: number;
  if (spreadBps >= 72)      spreadFactor = 1.15;
  else if (spreadBps >= 36) spreadFactor = 1.00;
  else if (spreadBps >= 20) spreadFactor = 0.85;
  else                      spreadFactor = 0.60;

  // Liquidity
  let liqFactor: number;
  if (liquidityUsd >= 1_000_000)    liqFactor = 1.00;
  else if (liquidityUsd >= 200_000) liqFactor = 0.90;
  else if (liquidityUsd >= 50_000)  liqFactor = 0.75;
  else                              liqFactor = 0.50;

  // Failure rate: 0% → 1.0×, 50% → 0.5×, 100% → 0.2×
  const failureFactor = Math.max(0.20, 1 - failureRate);

  // Network congestion (high fee = more MEV bot activity)
  const congestionFactor = 1 - 0.15 * Math.min(feeMicroLamports / 500_000, 1);

  // ── 3. Combine and clamp ─────────────────────────────────────────────────
  const raw = ceilingSol * spreadFactor * liqFactor * failureFactor * congestionFactor;
  const clamped = Math.max(MIN_SOL, Math.min(ceilingSol, raw));
  const solAmount = Math.round(clamped * 100) / 100;

  const finalBinding: FrontendSizeResult['binding'] =
    solAmount <= MIN_SOL ? 'min_clamp' : binding;

  const label =
    `${spreadBps}bps spread → ` +
    `MEV-safe ${mevSafeSol.toFixed(2)} SOL · ` +
    `optimal ${solAmount} SOL (no-Jito)`;

  return { solAmount, mevSafeSol, binding: finalBinding, label };
}
