//! Market microstructure models for Solana DEX execution cost estimation.
//!
//! All functions are pure (no I/O, no state) and operate on USD-denominated
//! inputs.  They are used to estimate round-trip execution costs before any
//! trade is evaluated by the filter chain.

// ─────────────────────────────────────────────────────────────────────────────
// Raydium AMM (CPMM x·y = k)
// ─────────────────────────────────────────────────────────────────────────────

/// Raydium constant-product AMM one-way price impact.
///
/// Returns the price-impact fraction for a single-leg swap of `trade_usd`
/// into a pool with total value locked `pool_tvl_usd`.
///
/// Derivation: for x·y = k, outputting Δy for input Δx gives
///   impact = Δx / (x + Δx)
/// where x ≈ pool_tvl_usd / 2 for a balanced 50/50 pool.
/// Simplified to `trade_usd / (pool_tvl_usd + trade_usd)`.
///
/// Returns 1.0 (maximum) when the pool has no liquidity.
pub fn raydium_price_impact(trade_usd: f64, pool_tvl_usd: f64) -> f64 {
    if pool_tvl_usd <= 0.0 {
        return 1.0;
    }
    trade_usd / (pool_tvl_usd + trade_usd)
}

// ─────────────────────────────────────────────────────────────────────────────
// Orca Whirlpool (CLMM)
// ─────────────────────────────────────────────────────────────────────────────

/// Orca Whirlpool concentrated-liquidity one-way price impact approximation.
///
/// `liquidity_in_range` is the USD liquidity within ±2 % of the current
/// price (the "active tick range").  Concentrated liquidity reduces impact
/// compared with CPMM because more capital is deployed near the current price.
///
/// Approximation: `impact = trade_usd / (2·liquidity_in_range + trade_usd)`
///
/// Returns 1.0 when `liquidity_in_range` is zero.
pub fn whirlpool_price_impact(trade_usd: f64, liquidity_in_range: f64) -> f64 {
    if liquidity_in_range <= 0.0 {
        return 1.0;
    }
    trade_usd / (2.0 * liquidity_in_range + trade_usd)
}

// ─────────────────────────────────────────────────────────────────────────────
// Jupiter routing cost
// ─────────────────────────────────────────────────────────────────────────────

/// Jupiter aggregator effective fee fraction.
///
/// Returns the total fee fraction (0.0–1.0) including per-hop DEX fees plus
/// a fixed routing-infrastructure overhead of 0.5 bps.
///
/// `num_hops` is typically 1 for a direct swap and 2–3 for routed paths.
/// `base_fee_bps` is the per-hop DEX fee (e.g. 25 for Raydium).
pub fn jupiter_effective_fee(trade_usd: f64, num_hops: u8, base_fee_bps: u16) -> f64 {
    let _ = trade_usd; // route selection is independent of size in this model
    let base = base_fee_bps as f64 / 10_000.0;
    let routing_overhead = 0.5 / 10_000.0;
    base * f64::from(num_hops) + routing_overhead
}

// ─────────────────────────────────────────────────────────────────────────────
// Round-trip cost
// ─────────────────────────────────────────────────────────────────────────────

/// Full round-trip execution cost in basis points.
///
/// Combines one-way Raydium price impact (doubled for entry + exit) and
/// Jupiter fees (doubled for both legs), returning the total cost expressed
/// in basis points.
pub fn round_trip_cost_bps(trade_usd: f64, pool_tvl_usd: f64, base_fee_bps: u16) -> f64 {
    let one_way_impact = raydium_price_impact(trade_usd, pool_tvl_usd);
    let rt_impact = one_way_impact * 2.0;

    let one_way_fee = jupiter_effective_fee(trade_usd, 1, base_fee_bps);
    let rt_fees = one_way_fee * 2.0;

    (rt_impact + rt_fees) * 10_000.0
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raydium_impact_is_zero_for_zero_trade() {
        assert_eq!(raydium_price_impact(0.0, 1_000_000.0), 0.0);
    }

    #[test]
    fn raydium_impact_is_one_for_zero_liquidity() {
        assert_eq!(raydium_price_impact(1_000.0, 0.0), 1.0);
    }

    #[test]
    fn raydium_impact_increases_with_trade_size() {
        let small = raydium_price_impact(100.0, 100_000.0);
        let large = raydium_price_impact(10_000.0, 100_000.0);
        assert!(large > small);
    }

    #[test]
    fn whirlpool_impact_lower_than_raydium_same_notional() {
        // CLMM with 50 % of TVL in range should beat CPMM.
        let tvl = 200_000.0;
        let in_range = tvl * 0.5;
        let trade = 1_000.0;
        let cpmm = raydium_price_impact(trade, tvl);
        let clmm = whirlpool_price_impact(trade, in_range);
        assert!(clmm < cpmm, "CLMM impact {clmm} should beat CPMM {cpmm}");
    }

    #[test]
    fn jupiter_fee_scales_with_hops() {
        let single = jupiter_effective_fee(1_000.0, 1, 25);
        let double = jupiter_effective_fee(1_000.0, 2, 25);
        assert!(double > single);
    }

    #[test]
    fn round_trip_cost_is_positive() {
        let cost = round_trip_cost_bps(1_000.0, 500_000.0, 25);
        assert!(cost > 0.0, "round-trip cost must be positive, got {cost}");
    }

    #[test]
    fn round_trip_cost_grows_with_trade_size() {
        let small = round_trip_cost_bps(100.0, 100_000.0, 25);
        let large = round_trip_cost_bps(10_000.0, 100_000.0, 25);
        assert!(large > small);
    }
}
