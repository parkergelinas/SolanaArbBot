//! Synchronous risk gate — stack-only, no allocation on approved path.

use super::precompute::PrecomputeTable;
use super::state::{HotState, PoolSlot};
use super::types::{HotSignal, RiskVerdict};

/// Evaluate all risk checks synchronously.
#[inline]
pub fn check_risk(
    signal: &HotSignal,
    pool: &PoolSlot,
    state: &HotState,
    table: &PrecomputeTable,
) -> RiskVerdict {
    let t = &table.thresholds;

    if !state.trading_enabled {
        return RiskVerdict::RejectedTradingDisabled;
    }

    if pool.liquidity_usd_x100 < t.min_liquidity_usd_x100 {
        return RiskVerdict::RejectedLiquidity;
    }

    if state.current_slot.saturating_sub(pool.last_trade_slot) < t.cooldown_slots {
        return RiskVerdict::RejectedCooldown;
    }

    if signal.edge_bps < t.min_edge_bps {
        return RiskVerdict::RejectedEdge;
    }

    let impact = pool.estimate_impact_bps(t.default_trade_lamports);
    let route = table.route(signal.route_idx);
    let total_cost_bps = impact + route.round_trip_fee_bps();
    if total_cost_bps > t.max_slippage_bps {
        return RiskVerdict::RejectedSlippage;
    }

    // Global inter-trade cooldown — prevents queue pile-up across all pools (item 5).
    if t.global_cooldown_slots > 0
        && state.current_slot.saturating_sub(state.last_any_intent_slot) < t.global_cooldown_slots
    {
        return RiskVerdict::RejectedGlobalCooldown;
    }

    let trade_usd_x100 = t.default_trade_usd_x100;
    if state.open_exposure_x100.saturating_add(trade_usd_x100) > t.max_exposure_x100 {
        return RiskVerdict::RejectedExposure;
    }

    // Per-trade absolute loss cap — matches the backtester guardrail (item 7).
    let worst_case_cost = (t.default_trade_lamports as u128 * total_cost_bps as u128 / 10_000) as u64;
    if worst_case_cost > t.max_loss_lamports {
        return RiskVerdict::RejectedMaxLoss;
    }

    RiskVerdict::Approved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotpath::types::HotSignal;
    use config::HotPathConfig;

    fn setup() -> (HotState, PrecomputeTable, HotSignal, PoolSlot) {
        let mut state = HotState::new();
        state.trading_enabled = true;
        state.current_slot = 1000;
        let table = PrecomputeTable::build(&HotPathConfig::default(), 1);
        let pool = PoolSlot {
            active: true,
            slot: 1000,
            liquidity_usd_x100: 500_000_00,
            reserve_a: 1_000_000_000_000,
            last_trade_slot: 0,
            ..PoolSlot::default()
        };
        let signal = HotSignal {
            pool_idx: 0,
            slot: 1000,
            edge_bps: 30,
            strength_x1000: 600,
            direction: 1,
            route_idx: 0,
        };
        (state, table, signal, pool)
    }

    #[test]
    fn approves_valid_signal() {
        let (state, table, signal, pool) = setup();
        assert_eq!(
            check_risk(&signal, &pool, &state, &table),
            RiskVerdict::Approved
        );
    }

    #[test]
    fn rejects_when_trading_disabled() {
        let (mut state, table, signal, pool) = setup();
        state.trading_enabled = false;
        assert_eq!(
            check_risk(&signal, &pool, &state, &table),
            RiskVerdict::RejectedTradingDisabled
        );
    }

    #[test]
    fn rejects_cooldown() {
        let (mut state, table, signal, mut pool) = setup();
        pool.last_trade_slot = 990;
        state.current_slot = 1000;
        assert_eq!(
            check_risk(&signal, &pool, &state, &table),
            RiskVerdict::RejectedCooldown
        );
    }
}
