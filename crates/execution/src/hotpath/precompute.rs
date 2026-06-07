//! Precomputation subsystem — all thresholds and routes resolved at startup.
//!
//! Nothing in this module runs in the hot loop except reading precomputed tables.

use config::HotPathConfig;

use super::types::{MAX_ROUTE_HOPS, Venue};

/// Fixed-point thresholds copied from config at init time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThresholdTable {
    pub min_edge_bps: u32,
    pub max_slippage_bps: u32,
    pub min_liquidity_usd_x100: u64,
    pub momentum_accel_x1000: u32,
    pub min_strength_x1000: u32,
    pub cooldown_slots: u64,
    /// Minimum slots between any two globally queued intents (global rate limiter).
    pub global_cooldown_slots: u64,
    pub max_exposure_x100: u64,
    pub default_trade_lamports: u64,
    pub default_trade_usd_x100: u64,
    /// Maximum acceptable loss in lamports per trade (per-trade circuit breaker).
    pub max_loss_lamports: u64,

    // ── Circuit breaker thresholds ────────────────────────────────────────────
    /// Halt if `consecutive_losses` reaches this many in a row. 0 = disabled.
    pub max_consecutive_losses: u32,
    /// Halt if `session_loss_lamports` reaches this total. 0 = disabled.
    pub max_session_loss_lamports: u64,

    // ── Velocity limiter thresholds ───────────────────────────────────────────
    /// Maximum intents allowed within `velocity_window_slots`. 0 = no limit.
    pub max_trades_per_window: u32,
    /// Width of the velocity window in slots (≈150 slots ≈ 1 minute).
    pub velocity_window_slots: u64,

    // ── Signal quality ────────────────────────────────────────────────────────
    /// Reject signals whose slot is more than this many slots behind `current_slot`.
    /// Prevents trading on stale price data. 0 = disabled. Default: 3 (~1.2s).
    pub max_signal_staleness_slots: u64,
    /// Actual trade size in lamports used for impact and worst-case cost estimates.
    /// Must match the size passed to the executor. Default: 100_000_000 (0.1 SOL).
    pub trade_size_lamports: u64,
}

impl ThresholdTable {
    /// Build from `SystemConfig` — called once at startup, never in hot loop.
    #[must_use]
    pub fn from_config(cfg: &HotPathConfig, max_exposure_usd_x100: u64) -> Self {
        // ~150 slots per minute at 400ms/slot.
        const SLOTS_PER_MINUTE: u64 = 150;
        let trade_lamports = cfg.trade_size_lamports;
        Self {
            min_edge_bps: cfg.min_edge_bps,
            max_slippage_bps: cfg.max_slippage_bps,
            min_liquidity_usd_x100: cfg.min_liquidity_usd_x100,
            momentum_accel_x1000: cfg.momentum_accel_threshold_x1000,
            min_strength_x1000: cfg.min_signal_strength_x1000,
            cooldown_slots: 75, // ~30s at 400ms/slot
            global_cooldown_slots: cfg.global_cooldown_slots,
            max_exposure_x100: max_exposure_usd_x100,
            default_trade_lamports: trade_lamports,
            default_trade_usd_x100: 200_00, // $200 notional
            max_loss_lamports: trade_lamports * cfg.max_loss_bps as u64 / 10_000,
            max_consecutive_losses: cfg.max_consecutive_losses,
            max_session_loss_lamports: cfg.max_session_loss_lamports,
            max_trades_per_window: cfg.max_trades_per_minute,
            velocity_window_slots: SLOTS_PER_MINUTE,
            max_signal_staleness_slots: cfg.max_signal_staleness_slots,
            trade_size_lamports: trade_lamports,
        }
    }
}

/// Precomputed swap route — no Jupiter API calls at runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrecomputedRoute {
    pub pool_idx: u16,
    pub venue: Venue,
    pub fee_bps: u16,
    /// Hop pool indices (0xFFFF = unused).
    pub hops: [u16; MAX_ROUTE_HOPS],
    pub hop_count: u8,
}

impl PrecomputedRoute {
    #[must_use]
    pub const fn direct_raydium(pool_idx: u16) -> Self {
        Self {
            pool_idx,
            venue: Venue::RaydiumAmm,
            fee_bps: 25,
            hops: [pool_idx, 0xFFFF],
            hop_count: 1,
        }
    }

    #[must_use]
    pub const fn direct_orca(pool_idx: u16) -> Self {
        Self {
            pool_idx,
            venue: Venue::OrcaWhirlpool,
            fee_bps: 5,
            hops: [pool_idx, 0xFFFF],
            hop_count: 1,
        }
    }

    /// Round-trip fee in basis points (entry + exit).
    #[inline]
    pub const fn round_trip_fee_bps(&self) -> u32 {
        (self.fee_bps as u32) * 2 * self.hop_count as u32
    }
}

/// All precomputed data for the hot loop.
#[derive(Clone, Debug)]
pub struct PrecomputeTable {
    pub thresholds: ThresholdTable,
    pub routes: Vec<PrecomputedRoute>,
    pub prefer_jito: bool,
    pub max_jito_tip_lamports: u64,
    pub allow_direct_rpc: bool,
    pub paper_mode: bool,
}

impl PrecomputeTable {
    /// Build route table for all registered pools at startup.
    #[must_use]
    pub fn build(cfg: &HotPathConfig, pool_count: u16) -> Self {
        let max_exposure = (cfg.min_liquidity_usd_x100 / 10).max(100_000_00);
        let thresholds = ThresholdTable::from_config(cfg, max_exposure);

        let mut routes = Vec::with_capacity(pool_count as usize * 2);
        for idx in 0..pool_count {
            routes.push(PrecomputedRoute::direct_raydium(idx));
            routes.push(PrecomputedRoute::direct_orca(idx));
        }

        Self {
            thresholds,
            routes,
            prefer_jito: cfg.prefer_jito,
            max_jito_tip_lamports: cfg.max_jito_tip_lamports,
            allow_direct_rpc: cfg.allow_direct_rpc_fallback,
            paper_mode: cfg.paper_mode,
        }
    }

    #[inline]
    pub fn route(&self, route_idx: u8) -> &PrecomputedRoute {
        &self.routes[route_idx as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raydium_route_has_50bps_rt_fee() {
        let route = PrecomputedRoute::direct_raydium(0);
        assert_eq!(route.round_trip_fee_bps(), 50);
    }

    #[test]
    fn orca_route_has_10bps_rt_fee() {
        let route = PrecomputedRoute::direct_orca(0);
        assert_eq!(route.round_trip_fee_bps(), 10);
    }
}
