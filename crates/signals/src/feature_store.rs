//! In-memory, async-safe feature store.
//!
//! # Design constraints (from `.cursor/rules.md`)
//!
//! * **No `Arc<RwLock<>>`** — DashMap provides per-shard sharded locking.
//! * **No blocking in async loops** — all entry mutations are O(1) amortised and
//!   hold the shard lock for microseconds.
//! * **Deterministic outputs** — `compute` is a pure function of store state + time.
//! * **No DB dependency** — data lives entirely in process memory.

use std::collections::VecDeque;

use common::{MarketEvent, Pubkey};
use config::SignalEngineConfig;
use dashmap::DashMap;

use crate::types::WhaleEvent;

// ─────────────────────────────────────────────────────────────────────────────
// Internal time-series entry
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Timed {
    ts: u64,  // microseconds
    val: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// PoolFeatures — per-pool rolling time series
// ─────────────────────────────────────────────────────────────────────────────

/// Raw time-series data stored per pool.  Mutated only through `FeatureStore`.
#[derive(Debug, Default)]
pub(crate) struct PoolFeatures {
    /// (timestamp_micros, sqrt_price) from PoolUpdate events.
    price: VecDeque<Timed>,
    /// (timestamp_micros, amount_in) from SwapEvent events.
    volume: VecDeque<Timed>,
    /// (timestamp_micros, liquidity) from PoolUpdate / TickUpdate events.
    liquidity: VecDeque<Timed>,
    /// Recent whale events, newest at the back.
    whale_events: VecDeque<WhaleEvent>,
}

impl PoolFeatures {
    fn push_price(&mut self, ts: u64, val: f64) {
        self.price.push_back(Timed { ts, val });
    }
    fn push_volume(&mut self, ts: u64, val: f64) {
        self.volume.push_back(Timed { ts, val });
    }
    fn push_liquidity(&mut self, ts: u64, val: f64) {
        self.liquidity.push_back(Timed { ts, val });
    }
    fn push_whale(&mut self, event: WhaleEvent) {
        self.whale_events.push_back(event);
    }

    /// Remove entries older than `cutoff_micros` from all series.
    fn prune(&mut self, cutoff_micros: u64) {
        prune_series(&mut self.price, cutoff_micros);
        prune_series(&mut self.volume, cutoff_micros);
        prune_series(&mut self.liquidity, cutoff_micros);
        while self
            .whale_events
            .front()
            .map_or(false, |e| e.timestamp_micros < cutoff_micros)
        {
            self.whale_events.pop_front();
        }
    }
}

fn prune_series(series: &mut VecDeque<Timed>, cutoff: u64) {
    while series.front().map_or(false, |t| t.ts < cutoff) {
        series.pop_front();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ComputedFeatures — output of a single `compute()` call
// ─────────────────────────────────────────────────────────────────────────────

/// All rolling metrics for one pool, computed on demand.
///
/// Processors consume this struct — they never read `PoolFeatures` directly.
#[derive(Clone, Debug)]
pub struct ComputedFeatures {
    /// Pool these features belong to.
    pub pool: Pubkey,
    /// Raw-unit volume sum over the short window (≈ 30 s).
    pub volume_short: f64,
    /// Raw-unit volume sum over the configured long window.
    pub volume_long: f64,
    /// (last_price − first_price) / first_price; 0.0 when < 2 samples.
    pub price_velocity: f64,
    /// (last_liq − first_liq) / first_liq; 0.0 when < 2 samples.
    pub liquidity_delta_pct: f64,
    /// tanh-normalised whale activity in [0.0, 1.0].
    pub whale_activity_score: f64,
    /// Mean profitability of qualifying smart-money events in [0.0, 1.0].
    pub smart_money_score: f64,
    /// Most recent price observation (0.0 if none).
    pub last_price: f64,
    /// Most recent liquidity observation (0.0 if none).
    pub last_liquidity: f64,
    /// Number of whale events in the long window.
    pub whale_event_count: usize,
    /// Total market data points (price + volume) in the long window.
    pub data_points: usize,
}

impl ComputedFeatures {
    /// Returns a zero-valued `ComputedFeatures` for a pool with no history.
    pub fn empty(pool: Pubkey) -> Self {
        Self {
            pool,
            volume_short: 0.0,
            volume_long: 0.0,
            price_velocity: 0.0,
            liquidity_delta_pct: 0.0,
            whale_activity_score: 0.0,
            smart_money_score: 0.0,
            last_price: 0.0,
            last_liquidity: 0.0,
            whale_event_count: 0,
            data_points: 0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FeatureStore
// ─────────────────────────────────────────────────────────────────────────────

/// Thread-safe in-memory store of rolling time-series features, keyed by pool.
///
/// All public methods take `&self` because `DashMap` handles internal locking.
/// No `Arc<RwLock<>>` is used anywhere in this type.
pub struct FeatureStore {
    pools: DashMap<Pubkey, PoolFeatures>,
    max_age_micros: u64,
}

impl FeatureStore {
    /// Creates a new store.  `max_age_secs` controls the rolling window kept in
    /// memory; entries older than this are pruned lazily on each update.
    pub fn new(max_age_secs: u64) -> Self {
        Self {
            pools: DashMap::new(),
            max_age_micros: max_age_secs.saturating_mul(1_000_000),
        }
    }

    /// Updates the store with a market event.
    ///
    /// Hot-path: holds the DashMap shard lock for < 1 µs on typical hardware.
    pub fn update_market(&self, event: &MarketEvent, now_micros: u64) {
        use common::{PoolUpdate, SwapEvent, TickUpdate};
        match event {
            MarketEvent::PoolUpdate(PoolUpdate { pool: Some(pool), sqrt_price, liquidity, .. }) => {
                let mut entry = self.pools.entry(*pool).or_default();
                if let Some(sp) = sqrt_price {
                    entry.push_price(now_micros, *sp as f64);
                }
                if let Some(liq) = liquidity {
                    entry.push_liquidity(now_micros, *liq as f64);
                }
                entry.prune(now_micros.saturating_sub(self.max_age_micros));
            }
            MarketEvent::SwapEvent(SwapEvent { pool, amount_in, .. }) => {
                let mut entry = self.pools.entry(*pool).or_default();
                entry.push_volume(now_micros, *amount_in as f64);
                entry.prune(now_micros.saturating_sub(self.max_age_micros));
            }
            MarketEvent::TickUpdate(TickUpdate { pool, liquidity_gross, .. }) => {
                let mut entry = self.pools.entry(*pool).or_default();
                entry.push_liquidity(now_micros, *liquidity_gross as f64);
                entry.prune(now_micros.saturating_sub(self.max_age_micros));
            }
            // PoolUpdate with no pool address — nothing to key on.
            MarketEvent::PoolUpdate(_) => {}
        }
    }

    /// Updates the store with a whale event.
    pub fn update_whale(&self, event: &WhaleEvent) {
        let ts = event.timestamp_micros;
        let mut entry = self.pools.entry(event.pool_address).or_default();
        entry.push_whale(event.clone());
        entry.prune(ts.saturating_sub(self.max_age_micros));
    }

    /// Computes rolling features for `pool` relative to `now_micros`.
    ///
    /// Returns `ComputedFeatures::empty` when the pool has no recorded history.
    pub fn compute(
        &self,
        pool: Pubkey,
        now_micros: u64,
        cfg: &SignalEngineConfig,
    ) -> ComputedFeatures {
        let long_window_micros = cfg.momentum_window_secs.saturating_mul(1_000_000);
        let short_window_micros: u64 = 30_000_000; // 30 s fixed short window

        let long_cutoff = now_micros.saturating_sub(long_window_micros);
        let short_cutoff = now_micros.saturating_sub(short_window_micros);

        let Some(entry) = self.pools.get(&pool) else {
            return ComputedFeatures::empty(pool);
        };

        let features: &PoolFeatures = &*entry;

        let volume_long: f64 = features
            .volume
            .iter()
            .filter(|t| t.ts >= long_cutoff && t.ts <= now_micros)
            .map(|t| t.val)
            .sum();

        let volume_short: f64 = features
            .volume
            .iter()
            .filter(|t| t.ts >= short_cutoff && t.ts <= now_micros)
            .map(|t| t.val)
            .sum();

        let price_velocity = velocity_bounded(&features.price, long_cutoff, now_micros);
        let liquidity_delta_pct = velocity_bounded(&features.liquidity, long_cutoff, now_micros);

        let last_price = features
            .price
            .iter()
            .filter(|t| t.ts <= now_micros)
            .last()
            .map_or(0.0, |t| t.val);
        let last_liquidity = features
            .liquidity
            .iter()
            .filter(|t| t.ts <= now_micros)
            .last()
            .map_or(0.0, |t| t.val);

        // Whale activity: sum(swap_usd / threshold) tanh-normalised.
        let whale_events_in_window: Vec<&WhaleEvent> = features
            .whale_events
            .iter()
            .filter(|e| e.timestamp_micros >= long_cutoff && e.timestamp_micros <= now_micros)
            .collect();

        let whale_event_count = whale_events_in_window.len();

        let raw_whale_score: f64 = whale_events_in_window
            .iter()
            .map(|e| e.swap_amount_usd / cfg.whale_threshold_usd)
            .sum();
        let whale_activity_score = raw_whale_score.tanh().clamp(0.0, 1.0);

        // Smart-money: mean profitability of qualifying events.
        let qualifying: Vec<f64> = whale_events_in_window
            .iter()
            .filter(|e| e.profitability_score >= cfg.smart_money_min_score)
            .map(|e| e.profitability_score)
            .collect();
        let smart_money_score = if qualifying.is_empty() {
            0.0
        } else {
            qualifying.iter().sum::<f64>() / qualifying.len() as f64
        };

        let data_points = features
            .price
            .iter()
            .filter(|t| t.ts >= long_cutoff && t.ts <= now_micros)
            .count()
            + features
                .volume
                .iter()
                .filter(|t| t.ts >= long_cutoff && t.ts <= now_micros)
                .count();

        ComputedFeatures {
            pool,
            volume_short,
            volume_long,
            price_velocity,
            liquidity_delta_pct,
            whale_activity_score,
            smart_money_score,
            last_price,
            last_liquidity,
            whale_event_count,
            data_points,
        }
    }

    /// Returns `true` if the store holds any data for `pool`.
    pub fn has_pool(&self, pool: Pubkey) -> bool {
        self.pools.contains_key(&pool)
    }

    /// Number of pools currently tracked.
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}

/// Computes (last − first) / first for entries with `cutoff <= ts <= now`.
/// Returns 0.0 when fewer than two samples are available.
fn velocity_bounded(series: &VecDeque<Timed>, cutoff: u64, now: u64) -> f64 {
    let in_window: Vec<f64> = series
        .iter()
        .filter(|t| t.ts >= cutoff && t.ts <= now)
        .map(|t| t.val)
        .collect();

    if in_window.len() < 2 {
        return 0.0;
    }
    let first = *in_window.first().unwrap();
    let last = *in_window.last().unwrap();
    if first == 0.0 {
        return 0.0;
    }
    (last - first) / first
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{Pubkey, SwapEvent, PoolUpdate};
    use config::SignalEngineConfig;

    fn cfg() -> SignalEngineConfig {
        SignalEngineConfig::default()
    }

    fn pool() -> Pubkey {
        Pubkey::new([1; 32])
    }

    #[test]
    fn empty_pool_returns_zero_features() {
        let store = FeatureStore::new(300);
        let f = store.compute(pool(), 1_000_000, &cfg());
        assert_eq!(f.data_points, 0);
        assert_eq!(f.volume_long, 0.0);
    }

    #[test]
    fn swap_events_accumulate_volume() {
        let store = FeatureStore::new(300);
        let p = pool();
        let now = 100_000_000_u64;

        for i in 0..5u128 {
            let evt = MarketEvent::SwapEvent(SwapEvent {
                pool: p,
                input_mint: Pubkey::new([0; 32]),
                output_mint: Pubkey::new([0; 32]),
                amount_in: 1_000 * (i + 1),
                amount_out: 999,
            });
            store.update_market(&evt, now + i as u64 * 1_000_000);
        }

        let f = store.compute(p, now + 4_000_000, &cfg());
        // 1000 + 2000 + 3000 + 4000 + 5000 = 15000
        assert!((f.volume_long - 15_000.0).abs() < 1.0);
    }

    #[test]
    fn pool_update_without_pool_is_ignored() {
        let store = FeatureStore::new(300);
        let evt = MarketEvent::PoolUpdate(PoolUpdate {
            pool: None,
            token_a_mint: None,
            token_b_mint: None,
            liquidity: Some(100),
            sqrt_price: Some(200),
            fee_rate: None,
        });
        store.update_market(&evt, 1_000_000);
        assert_eq!(store.pool_count(), 0);
    }

    #[test]
    fn whale_activity_score_is_normalised() {
        let store = FeatureStore::new(300);
        let p = pool();
        let now = 100_000_000_u64;

        // Single whale event at 5× threshold.
        store.update_whale(&WhaleEvent {
            timestamp_micros: now,
            pool_address: p,
            swap_amount_usd: 50_000.0,
            direction: crate::types::Direction::Long,
            profitability_score: 0.8,
        });

        let f = store.compute(p, now + 1_000, &cfg());
        assert!(f.whale_activity_score > 0.0);
        assert!(f.whale_activity_score <= 1.0);
    }

    #[test]
    fn price_velocity_positive_on_rising_prices() {
        let store = FeatureStore::new(300);
        let p = pool();
        let now = 100_000_000_u64;

        // Rising sqrt_price: 1000, 1100, 1200
        for (i, sp) in [1_000_u128, 1_100, 1_200].iter().enumerate() {
            let evt = MarketEvent::PoolUpdate(PoolUpdate {
                pool: Some(p),
                token_a_mint: None,
                token_b_mint: None,
                liquidity: None,
                sqrt_price: Some(*sp),
                fee_rate: None,
            });
            store.update_market(&evt, now + i as u64 * 5_000_000);
        }

        let f = store.compute(p, now + 15_000_000, &cfg());
        assert!(f.price_velocity > 0.0, "expected positive velocity, got {}", f.price_velocity);
    }

    #[test]
    fn old_entries_pruned_outside_window() {
        let store = FeatureStore::new(10); // 10 s window
        let p = pool();

        let old = 0_u64;
        let now = 20_000_000_u64; // 20 s later → old entry is outside 10 s window

        let evt = MarketEvent::SwapEvent(SwapEvent {
            pool: p,
            input_mint: Pubkey::new([0; 32]),
            output_mint: Pubkey::new([0; 32]),
            amount_in: 9_999,
            amount_out: 1,
        });
        store.update_market(&evt, old);

        // Force prune by adding a fresh event.
        let fresh = MarketEvent::SwapEvent(SwapEvent {
            pool: p,
            input_mint: Pubkey::new([0; 32]),
            output_mint: Pubkey::new([0; 32]),
            amount_in: 1,
            amount_out: 1,
        });
        store.update_market(&fresh, now);

        let f = store.compute(p, now, &cfg());
        // Only the fresh event (amount_in=1) is inside the 60 s long window.
        assert_eq!(f.volume_long, 1.0, "old entry should be pruned: got {}", f.volume_long);
    }
}
