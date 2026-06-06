//! In-memory pool state — preallocated, stack-friendly, minimal locking.
//!
//! The hot loop reads/writes `PoolSlot` entries by index.  No hash maps,
//! no `DashMap`, no `Arc<RwLock<>>` on the hot path.

use super::types::{MarketTick, PoolIdx, MAX_POOLS, PRICE_SCALE};

/// Single pool slot — entirely `Copy`, updated in-place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoolSlot {
    pub active: bool,
    pub slot: u64,
    pub price_fp: u64,
    pub reserve_a: u64,
    pub reserve_b: u64,
    /// Rolling short-window volume (USD × 100).
    pub volume_short_x100: u64,
    /// Rolling long-window volume (USD × 100).
    pub volume_long_x100: u64,
    /// Previous price for velocity computation.
    pub prev_price_fp: u64,
    /// Last trade slot — used for per-pool cooldown.
    pub last_trade_slot: u64,
    /// Liquidity estimate USD × 100.
    pub liquidity_usd_x100: u64,
    /// Venue tag (0 = Raydium, 1 = Orca CLMM).
    pub venue: u8,
}

impl PoolSlot {
    /// Apply a market tick, updating rolling features in-place.
    #[inline]
    pub fn apply_tick(&mut self, tick: &MarketTick) {
        if !self.active {
            self.active = true;
            self.prev_price_fp = tick.price_fp;
        }

        self.slot = tick.slot;
        self.prev_price_fp = self.price_fp;
        self.price_fp = tick.price_fp;
        self.reserve_a = tick.reserve_a;
        self.reserve_b = tick.reserve_b;

        // Exponential decay volume windows (integer arithmetic).
        self.volume_short_x100 = self.volume_short_x100 * 9 / 10 + tick.volume_delta_usd_x100;
        self.volume_long_x100 = self.volume_long_x100 * 99 / 100 + tick.volume_delta_usd_x100;

        // Liquidity proxy: 2 × min side reserve value (simplified).
        let side_a_usd = tick.reserve_a.saturating_mul(tick.price_fp) / PRICE_SCALE;
        let side_b_usd = tick.reserve_b;
        let min_side = side_a_usd.min(side_b_usd);
        self.liquidity_usd_x100 = min_side.saturating_mul(2).min(u64::MAX / 100) * 100;
    }

    /// Price velocity as fixed-point ratio delta (×10000 = bps-scale).
    #[inline]
    pub fn price_velocity_x10000(&self) -> i64 {
        if self.prev_price_fp == 0 {
            return 0;
        }
        let delta = self.price_fp as i128 - self.prev_price_fp as i128;
        (delta * 10_000 / self.prev_price_fp as i128) as i64
    }

    /// Volume acceleration: short_rate / long_rate × 1000.
    #[inline]
    pub fn volume_accel_x1000(&self) -> u32 {
        if self.volume_long_x100 == 0 {
            return 0;
        }
        // Short window ~10 ticks, long ~100 — normalize rates.
        let short_rate = self.volume_short_x100 * 10;
        let long_rate = self.volume_long_x100;
        if long_rate == 0 {
            return 0;
        }
        ((short_rate * 1000) / long_rate).min(u32::MAX as u64) as u32
    }

    /// One-leg AMM price impact in basis points (CPMM approximation).
    #[inline]
    pub fn estimate_impact_bps(&self, trade_size: u64) -> u32 {
        if self.reserve_a == 0 {
            return u32::MAX;
        }
        // impact ≈ trade / (reserve + trade) × 10000
        let impact = (trade_size as u128 * 10_000) / (self.reserve_a as u128 + trade_size as u128);
        impact.min(u32::MAX as u128) as u32
    }
}

/// Preallocated pool table — the sole in-memory state for the hot loop.
#[derive(Clone, Debug)]
pub struct HotState {
    pub pools: [PoolSlot; MAX_POOLS],
    pub pool_count: PoolIdx,
    pub current_slot: u64,
    pub trading_enabled: bool,
    pub open_exposure_x100: u64,
    /// Slot of the most recently queued intent across all pools (global rate limiter).
    pub last_any_intent_slot: u64,
}

impl Default for HotState {
    fn default() -> Self {
        Self {
            pools: [PoolSlot::default(); MAX_POOLS],
            pool_count: 0,
            current_slot: 0,
            trading_enabled: false,
            open_exposure_x100: 0,
            last_any_intent_slot: 0,
        }
    }
}

impl HotState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pool and return its index.
    pub fn register_pool(&mut self, venue: u8) -> Option<PoolIdx> {
        let idx = self.pool_count;
        if idx as usize >= MAX_POOLS {
            return None;
        }
        self.pools[idx as usize].active = true;
        self.pools[idx as usize].venue = venue;
        self.pool_count = idx.saturating_add(1);
        Some(idx)
    }

    #[inline]
    pub fn pool(&self, idx: PoolIdx) -> &PoolSlot {
        &self.pools[idx as usize]
    }

    #[inline]
    pub fn pool_mut(&mut self, idx: PoolIdx) -> &mut PoolSlot {
        &mut self.pools[idx as usize]
    }

    /// Process one tick synchronously — updates state, returns updated slot ref.
    #[inline]
    pub fn ingest(&mut self, tick: &MarketTick) -> &PoolSlot {
        self.current_slot = tick.slot;
        let slot = self.pool_mut(tick.pool_idx);
        slot.apply_tick(tick);
        // SAFETY: we just mutably borrowed; re-borrow immutably via index.
        &self.pools[tick.pool_idx as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_tick_updates_liquidity() {
        let mut state = HotState::new();
        let idx = state.register_pool(0).expect("register");
        let tick = MarketTick {
            pool_idx: idx,
            slot: 100,
            price_fp: PRICE_SCALE,
            reserve_a: 1_000_000_000,
            reserve_b: 1_000_000_000,
            volume_delta_usd_x100: 500_00,
        };
        state.ingest(&tick);
        assert!(state.pool(idx).liquidity_usd_x100 > 0);
    }

    #[test]
    fn volume_accel_detects_spike() {
        let mut slot = PoolSlot {
            active: true,
            volume_long_x100: 10_000_00,
            volume_short_x100: 5_000_00,
            ..PoolSlot::default()
        };
        slot.volume_short_x100 = 8_000_00;
        assert!(slot.volume_accel_x1000() > 1000);
    }
}
