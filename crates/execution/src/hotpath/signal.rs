//! Inline micro-momentum signal evaluator — feature-store logic, no heap.

use super::precompute::ThresholdTable;
use super::state::PoolSlot;
use super::types::HotSignal;

/// Evaluate a micro-momentum signal from precomputed pool features.
///
/// Returns `None` when no actionable edge is detected.
#[inline]
pub fn evaluate_momentum(
    pool: &PoolSlot,
    pool_idx: u16,
    route_idx: u8,
    thresholds: &ThresholdTable,
) -> Option<HotSignal> {
    if !pool.active {
        return None;
    }

    let accel = pool.volume_accel_x1000();
    if accel < thresholds.momentum_accel_x1000 {
        return None;
    }

    let velocity = pool.price_velocity_x10000();
    if velocity == 0 {
        return None;
    }

    // Strength from acceleration excess over threshold.
    let excess = accel.saturating_sub(thresholds.momentum_accel_x1000);
    let strength_x1000 = (400 + excess / 2).min(1000);
    if strength_x1000 < thresholds.min_strength_x1000 {
        return None;
    }

    // Edge estimate: strength-scaled bps minus venue fee floor (~15 bps Orca).
    let gross_edge_bps = (strength_x1000 / 10) as u32 + 10;
    let edge_bps = gross_edge_bps.saturating_sub(15);
    if edge_bps < thresholds.min_edge_bps {
        return None;
    }

    let direction = if velocity > 0 { 1u8 } else { 2u8 };

    Some(HotSignal {
        pool_idx,
        slot: pool.slot,
        edge_bps,
        strength_x1000,
        direction,
        route_idx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotpath::types::PRICE_SCALE;

    fn thresholds() -> ThresholdTable {
        ThresholdTable {
            min_edge_bps: 20,
            max_slippage_bps: 50,
            min_liquidity_usd_x100: 100_000_00,
            momentum_accel_x1000: 1100,
            min_strength_x1000: 400,
            cooldown_slots: 75,
            max_exposure_x100: 2_000_00,
            default_trade_lamports: 100_000_000,
            default_trade_usd_x100: 200_00,
        }
    }

    #[test]
    fn no_signal_on_flat_pool() {
        let pool = PoolSlot {
            active: true,
            price_fp: PRICE_SCALE,
            prev_price_fp: PRICE_SCALE,
            ..PoolSlot::default()
        };
        assert!(evaluate_momentum(&pool, 0, 0, &thresholds()).is_none());
    }

    #[test]
    fn fires_on_volume_spike_with_price_move() {
        let pool = PoolSlot {
            active: true,
            price_fp: PRICE_SCALE + PRICE_SCALE / 100,
            prev_price_fp: PRICE_SCALE,
            volume_short_x100: 50_000_00,
            volume_long_x100: 10_000_00,
            ..PoolSlot::default()
        };
        assert!(evaluate_momentum(&pool, 0, 0, &thresholds()).is_some());
    }
}
