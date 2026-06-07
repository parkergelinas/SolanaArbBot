/**
 * MEV-aware dynamic trade sizer.
 *
 * Core insight: the binding constraint on trade size is not fee dilution (fixed
 * costs per tx are ~$0.002 — negligible) but the MEV sandwich threshold.
 *
 * A sandwich bot profits when:
 *   extractable_value = trade_size × spread_bps/10000 × sol_price_usd > MEV_THRESHOLD
 *
 * Solving for trade_size gives the MEV-safe ceiling for any (spread, sol_price) pair:
 *   mev_safe_sol = MEV_THRESHOLD / (spread_bps / 10_000 × sol_price_usd)
 *
 * With Jito bundle protection active, sandwich attacks are neutralised so we
 * substitute a higher fixed ceiling (JITO_CEILING_SOL) instead.
 *
 * Secondary factors (liquidity depth, failure rate, congestion, route quality)
 * are then applied multiplicatively *downward* from the ceiling — they can only
 * shrink the trade, never push it above the MEV-safe limit.
 */
// ── Constants ─────────────────────────────────────────────────────────────────
/**
 * USD value at which a sandwich becomes profitable for the attacker.
 * Below this, the tip cost to acquire a leader slot exceeds the extractable value.
 * Derived from observed Solana MEV bot economics at typical block times.
 */
const MEV_THRESHOLD_USD = 0.50;
/**
 * Hard ceiling when Jito bundle protection is active.
 * Sandwiching a Jito bundle requires bribing the leader at a cost that typically
 * exceeds the extractable value — making this ceiling effectively MEV-proof.
 */
const JITO_CEILING_SOL = 0.80;
/**
 * Fallback ceiling without Jito. Conservative enough that even at the minimum
 * detected spread (36 bps) and a $180 SOL price the MEV-safe limit is ~0.77 SOL,
 * so this floor never overrides the equation.
 */
const NO_JITO_CEILING_SOL = 0.42;
// ── Main function ─────────────────────────────────────────────────────────────
/**
 * Compute the optimal trade size for a single opportunity.
 *
 * Returns a {@link SizingResult} — use `result.amountUi` for the atomic
 * lamport conversion and `result.rationale` for structured logging.
 */
export function computeDynamicSize(input) {
    const { spreadBps, liquidityUsd, routeQualityScore, recentFailureRate, priorityFeeMicroLamports, solPriceUsd, jitoActive, minAmountUi, maxAmountUi, } = input;
    // ── 1. MEV sandwich ceiling ───────────────────────────────────────────────
    //
    // Solve: MEV_THRESHOLD = size × (spread/10000) × sol_price  for  size.
    // If we can't compute (zero spread or zero price), fall back to the
    // conservative no-jito ceiling.
    let mevSafeSol;
    if (solPriceUsd > 0 && spreadBps > 0) {
        mevSafeSol = MEV_THRESHOLD_USD / ((spreadBps / 10_000) * solPriceUsd);
    }
    else {
        mevSafeSol = NO_JITO_CEILING_SOL;
    }
    // ── 2. Effective ceiling (Jito overrides the MEV equation) ───────────────
    let ceilingSol;
    let ceilingLabel;
    if (jitoActive) {
        ceilingSol = JITO_CEILING_SOL;
        ceilingLabel = 'jito';
    }
    else if (mevSafeSol < NO_JITO_CEILING_SOL) {
        // Wide spread → low MEV-safe size.  The equation is the tighter bound.
        ceilingSol = mevSafeSol;
        ceilingLabel = 'mev_threshold';
    }
    else {
        // Narrow spread or deep liquidity → conservative default is the ceiling.
        ceilingSol = NO_JITO_CEILING_SOL;
        ceilingLabel = 'mev_threshold';
    }
    // Hard config cap (safety net for misconfiguration)
    if (ceilingSol > maxAmountUi) {
        ceilingSol = maxAmountUi;
        ceilingLabel = 'config_max';
    }
    // ── 3. Spread confidence factor ───────────────────────────────────────────
    // Wider spread = more confident the opportunity survives network latency.
    // This factor can push UP toward the ceiling but cannot exceed it.
    let spreadFactor;
    if (spreadBps >= 72)
        spreadFactor = 1.15; // very wide — take the full ceiling
    else if (spreadBps >= 36)
        spreadFactor = 1.00; // healthy spread
    else if (spreadBps >= 20)
        spreadFactor = 0.85; // marginal — trade smaller
    else
        spreadFactor = 0.60; // near break-even — minimum size only
    // ── 4. Liquidity depth factor ─────────────────────────────────────────────
    // Don't push large size into a thin pool — price impact would eat the spread.
    let liqFactor;
    if (liquidityUsd >= 1_000_000)
        liqFactor = 1.00;
    else if (liquidityUsd >= 200_000)
        liqFactor = 0.90;
    else if (liquidityUsd >= 50_000)
        liqFactor = 0.75;
    else
        liqFactor = 0.50;
    // ── 5. Recent failure rate penalty ───────────────────────────────────────
    // Shrink size when execution has been unreliable. Recovers as successes
    // accumulate (engine decays failureRate by −0.1 per success).
    // 0% failures → 1.0×; 50% → 0.5×; 100% → 0.2×
    const failureFactor = Math.max(0.20, 1 - recentFailureRate);
    // ── 6. Network congestion factor ─────────────────────────────────────────
    // High priority fees indicate congestion where MEV bots are most active.
    // At 500k µ-lamports (very high) → 15% reduction.
    const congestionFactor = 1 - 0.15 * Math.min(priorityFeeMicroLamports / 500_000, 1);
    // ── 7. Route quality factor ───────────────────────────────────────────────
    // Poor routes have higher fail risk and worse slippage.
    // score 0 → 0.60×; score 1 → 1.00×
    const qualityFactor = 0.60 + routeQualityScore * 0.40;
    // ── 8. Combine: start from ceiling, apply downward multipliers ────────────
    const raw = ceilingSol *
        spreadFactor *
        liqFactor *
        failureFactor *
        congestionFactor *
        qualityFactor;
    // ── 9. Clamp to [min, ceiling] and round to 2 dp ─────────────────────────
    const clamped = Math.max(minAmountUi, Math.min(ceilingSol, raw));
    const amountUi = Math.round(clamped * 100) / 100;
    if (amountUi <= minAmountUi) {
        ceilingLabel = 'min_clamp';
    }
    const rationale = `spread=${spreadBps}bps sol=$${solPriceUsd} ` +
        `mev_safe=${mevSafeSol.toFixed(2)}SOL ` +
        `ceiling=${ceilingSol.toFixed(2)}SOL(${jitoActive ? 'jito' : 'no-jito'}) ` +
        `factors=[sp:${spreadFactor.toFixed(2)} ` +
        `liq:${liqFactor.toFixed(2)} ` +
        `fail:${failureFactor.toFixed(2)} ` +
        `cong:${congestionFactor.toFixed(2)} ` +
        `qual:${qualityFactor.toFixed(2)}] ` +
        `→ ${amountUi}SOL(${ceilingLabel})`;
    return { amountUi, ceiling: ceilingLabel, mevSafeSol, rationale };
}
// ── Convenience re-export (preserves old call-sites that destructure result) ─
/** @deprecated Use computeDynamicSize and read result.amountUi */
export function computeDynamicSizeUi(input) {
    return computeDynamicSize(input).amountUi;
}
