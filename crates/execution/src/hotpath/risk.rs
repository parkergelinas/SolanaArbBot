//! Synchronous risk gate — stack-only, no allocation on approved path.

use super::precompute::PrecomputeTable;
use super::state::{HotState, PoolSlot};
use super::types::{HotSignal, RiskVerdict};

/// Evaluate all risk checks synchronously.
///
/// Order is arranged to short-circuit cheaply before more expensive checks:
/// 1. Hard stops (disabled, circuit breaker)
/// 2. Market quality (liquidity, edge, slippage)
/// 3. Rate limits (cooldown, velocity)
/// 4. Portfolio limits (exposure, session loss)
#[inline]
pub fn check_risk(
    signal: &HotSignal,
    pool: &PoolSlot,
    state: &HotState,
    table: &PrecomputeTable,
) -> RiskVerdict {
    let t = &table.thresholds;

    // ── Hard stops ────────────────────────────────────────────────────────────

    if !state.trading_enabled {
        return RiskVerdict::RejectedTradingDisabled;
    }

    // Circuit breaker: tripped by consecutive losses or session loss cap.
    if state.circuit_breaker_tripped {
        return RiskVerdict::RejectedCircuitBreaker;
    }

    // Session loss cap (evaluated here for latency; already set by cold path).
    if t.max_session_loss_lamports > 0
        && state.session_loss_lamports >= t.max_session_loss_lamports
    {
        return RiskVerdict::RejectedCircuitBreaker;
    }

    // Consecutive losses circuit breaker.
    if t.max_consecutive_losses > 0 && state.consecutive_losses >= t.max_consecutive_losses {
        return RiskVerdict::RejectedCircuitBreaker;
    }

    // ── Market quality ────────────────────────────────────────────────────────

    if pool.liquidity_usd_x100 < t.min_liquidity_usd_x100 {
        return RiskVerdict::RejectedLiquidity;
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

    // ── Rate limits ───────────────────────────────────────────────────────────

    if state.current_slot.saturating_sub(pool.last_trade_slot) < t.cooldown_slots {
        return RiskVerdict::RejectedCooldown;
    }

    // Global inter-trade cooldown — prevents queue pile-up across all pools.
    if t.global_cooldown_slots > 0
        && state.current_slot.saturating_sub(state.last_any_intent_slot)
            < t.global_cooldown_slots
    {
        return RiskVerdict::RejectedGlobalCooldown;
    }

    // Velocity cap: max trades per rolling window (default = 1 minute).
    if t.max_trades_per_window > 0 {
        let window_elapsed = state
            .current_slot
            .saturating_sub(state.velocity_window_start_slot);
        let in_window = window_elapsed < t.velocity_window_slots;
        if in_window && state.velocity_trades_this_window >= t.max_trades_per_window {
            return RiskVerdict::RejectedVelocityLimit;
        }
    }

    // ── Portfolio limits ──────────────────────────────────────────────────────

    let trade_usd_x100 = t.default_trade_usd_x100;
    if state.open_exposure_x100.saturating_add(trade_usd_x100) > t.max_exposure_x100 {
        return RiskVerdict::RejectedExposure;
    }

    // Per-trade absolute loss cap.
    let worst_case_cost =
        (t.default_trade_lamports as u128 * total_cost_bps as u128 / 10_000) as u64;
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

    #[test]
    fn rejects_when_circuit_breaker_tripped() {
        let (mut state, table, signal, pool) = setup();
        state.circuit_breaker_tripped = true;
        assert_eq!(
            check_risk(&signal, &pool, &state, &table),
            RiskVerdict::RejectedCircuitBreaker
        );
    }

    #[test]
    fn rejects_on_consecutive_loss_limit() {
        let (mut state, mut table, signal, pool) = setup();
        table.thresholds.max_consecutive_losses = 3;
        state.consecutive_losses = 3;
        assert_eq!(
            check_risk(&signal, &pool, &state, &table),
            RiskVerdict::RejectedCircuitBreaker
        );
    }

    #[test]
    fn rejects_on_session_loss_cap() {
        let (mut state, mut table, signal, pool) = setup();
        table.thresholds.max_session_loss_lamports = 1_000_000_000; // 1 SOL
        state.session_loss_lamports = 1_000_000_000;
        assert_eq!(
            check_risk(&signal, &pool, &state, &table),
            RiskVerdict::RejectedCircuitBreaker
        );
    }

    #[test]
    fn rejects_on_velocity_limit() {
        let (mut state, mut table, signal, pool) = setup();
        table.thresholds.max_trades_per_window = 5;
        table.thresholds.velocity_window_slots = 150;
        state.velocity_window_start_slot = 900;
        state.velocity_trades_this_window = 5; // at the limit
        state.current_slot = 1000; // 100 slots into window (< 150)
        assert_eq!(
            check_risk(&signal, &pool, &state, &table),
            RiskVerdict::RejectedVelocityLimit
        );
    }

    #[test]
    fn velocity_window_resets_after_expiry() {
        // If velocity_window_start_slot is old enough, the window is expired
        // and a new window would start — so velocity check passes.
        let (mut state, mut table, signal, pool) = setup();
        table.thresholds.max_trades_per_window = 5;
        table.thresholds.velocity_window_slots = 150;
        state.velocity_window_start_slot = 100; // 900 slots ago
        state.velocity_trades_this_window = 100; // lots of old trades
        state.current_slot = 1000;
        // Window elapsed = 900 >= 150 → window expired → not in_window → passes
        assert_eq!(
            check_risk(&signal, &pool, &state, &table),
            RiskVerdict::Approved
        );
    }
}
