//! Trade filter chain — the CRITICAL module of the scalper.
//!
//! Every filter implements [`TradeFilter`] with a `check` method that returns
//! [`FilterResult::Pass`] or [`FilterResult::Reject`].  Stateful filters
//! (cooldown, rate limit) also implement `record_trade` which is called by
//! `ScalpEngine` after a successful paper fill.
//!
//! # Filter evaluation order
//!
//! 1. `LiquidityFilter`       — pool TVL + rolling volume gate
//! 2. `VolatilityRegimeFilter` — price-velocity z-score gate
//! 3. `SlippageFilter`        — round-trip cost ceiling
//! 4. `CooldownFilter`        — per-pool re-trade cooldown (DashMap)
//! 5. `RateLimitFilter`       — rolling-hour per-asset + global counters
//! 6. `MinEdgeFilter`         — net-edge floor
//!
//! No `Arc<RwLock<>>` is used anywhere; `DashMap` provides per-shard locking
//! and `std::sync::Mutex` is used only for the global trade-counter deque.

use std::collections::VecDeque;
use std::sync::Mutex;

use common::Pubkey;
use config::ScalerConfig;
use dashmap::DashMap;
use signals::ComputedFeatures;

use crate::candidate::TradeCandidate;

// ─────────────────────────────────────────────────────────────────────────────
// Filter trait and result
// ─────────────────────────────────────────────────────────────────────────────

/// Result returned by every [`TradeFilter::check`] call.
pub enum FilterResult {
    Pass,
    Reject { reason: &'static str },
}

/// A single stage in the trade filter pipeline.
///
/// Implementations must be `Send + Sync` (the filter chain lives behind `Arc`
/// inside `ScalpEngine`).  The default `record_trade` no-op is overridden only
/// by stateful filters that need to update their internal counters on fill.
pub trait TradeFilter: Send + Sync {
    /// Short, stable name used in rejection log lines.
    fn name(&self) -> &'static str;

    /// Evaluate the candidate.  `now_micros` is the caller's view of wall time.
    fn check(
        &self,
        candidate: &TradeCandidate,
        features: &ComputedFeatures,
        now_micros: u64,
    ) -> FilterResult;

    /// Record that a trade was executed for `pool` at `now_micros`.
    ///
    /// Called by the engine *after* a successful paper fill so that stateful
    /// filters (cooldown, rate limit) can update their internal state.
    fn record_trade(&self, _pool: Pubkey, _now_micros: u64) {}
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. LiquidityFilter
// ─────────────────────────────────────────────────────────────────────────────

struct LiquidityFilter {
    min_tvl: f64,
    min_volume: f64,
}

impl TradeFilter for LiquidityFilter {
    fn name(&self) -> &'static str {
        "LiquidityFilter"
    }

    fn check(
        &self,
        candidate: &TradeCandidate,
        features: &ComputedFeatures,
        _now_micros: u64,
    ) -> FilterResult {
        if candidate.pool_tvl_usd < self.min_tvl {
            return FilterResult::Reject {
                reason: "pool_tvl_usd below min_pool_tvl_usd",
            };
        }
        // volume_long is used as a proxy for 5-minute rolling volume.
        if features.volume_long < self.min_volume {
            return FilterResult::Reject {
                reason: "rolling_volume below min_volume_5m_usd",
            };
        }
        FilterResult::Pass
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. VolatilityRegimeFilter
// ─────────────────────────────────────────────────────────────────────────────

struct VolatilityRegimeFilter {
    floor: f64,
    ceiling: f64,
}

impl TradeFilter for VolatilityRegimeFilter {
    fn name(&self) -> &'static str {
        "VolatilityRegimeFilter"
    }

    fn check(
        &self,
        _candidate: &TradeCandidate,
        features: &ComputedFeatures,
        _now_micros: u64,
    ) -> FilterResult {
        // Treat |price_velocity| as the dimensionless volatility proxy.
        let zscore = features.price_velocity.abs();
        if zscore < self.floor {
            return FilterResult::Reject {
                reason: "price_velocity below volatility_floor (dead market)",
            };
        }
        if zscore > self.ceiling {
            return FilterResult::Reject {
                reason: "price_velocity above volatility_ceiling (excessive volatility)",
            };
        }
        FilterResult::Pass
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. SlippageFilter
// ─────────────────────────────────────────────────────────────────────────────

struct SlippageFilter {
    max_rt_cost_bps: f64,
}

impl TradeFilter for SlippageFilter {
    fn name(&self) -> &'static str {
        "SlippageFilter"
    }

    fn check(
        &self,
        candidate: &TradeCandidate,
        _features: &ComputedFeatures,
        _now_micros: u64,
    ) -> FilterResult {
        if candidate.estimated_round_trip_cost_bps > self.max_rt_cost_bps {
            return FilterResult::Reject {
                reason: "estimated_round_trip_cost_bps exceeds max_slippage_bps",
            };
        }
        FilterResult::Pass
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. CooldownFilter
// ─────────────────────────────────────────────────────────────────────────────

struct CooldownFilter {
    /// Cooldown window in microseconds.
    cooldown_micros: u64,
    /// Per-pool last-trade timestamp.  DashMap provides lock-free reads/writes.
    last_trade: DashMap<Pubkey, u64>,
}

impl TradeFilter for CooldownFilter {
    fn name(&self) -> &'static str {
        "CooldownFilter"
    }

    fn check(
        &self,
        candidate: &TradeCandidate,
        _features: &ComputedFeatures,
        now_micros: u64,
    ) -> FilterResult {
        if let Some(last_ts) = self.last_trade.get(&candidate.pool_address) {
            if last_ts.saturating_add(self.cooldown_micros) > now_micros {
                return FilterResult::Reject {
                    reason: "pool is within trade cooldown window",
                };
            }
        }
        FilterResult::Pass
    }

    fn record_trade(&self, pool: Pubkey, now_micros: u64) {
        self.last_trade.insert(pool, now_micros);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. RateLimitFilter
// ─────────────────────────────────────────────────────────────────────────────

const ONE_HOUR_MICROS: u64 = 3_600_000_000;

struct RateLimitFilter {
    max_per_asset: u32,
    max_global: u32,
    /// Per-pool rolling trade timestamps (1-hour window).
    per_asset: DashMap<Pubkey, VecDeque<u64>>,
    /// Global rolling trade timestamps (1-hour window).
    global: Mutex<VecDeque<u64>>,
}

impl TradeFilter for RateLimitFilter {
    fn name(&self) -> &'static str {
        "RateLimitFilter"
    }

    fn check(
        &self,
        candidate: &TradeCandidate,
        _features: &ComputedFeatures,
        now_micros: u64,
    ) -> FilterResult {
        let cutoff = now_micros.saturating_sub(ONE_HOUR_MICROS);

        // Per-asset: read without modification (conservative — stale entries
        // only over-count, keeping the limit safe).
        let asset_count = self
            .per_asset
            .get(&candidate.pool_address)
            .map(|e| e.iter().filter(|&&ts| ts >= cutoff).count())
            .unwrap_or(0);

        if asset_count >= self.max_per_asset as usize {
            return FilterResult::Reject {
                reason: "per-asset hourly trade limit reached",
            };
        }

        // Global: brief Mutex lock.
        let global_count = {
            let g = self.global.lock().unwrap();
            g.iter().filter(|&&ts| ts >= cutoff).count()
        };

        if global_count >= self.max_global as usize {
            return FilterResult::Reject {
                reason: "global hourly trade limit reached",
            };
        }

        FilterResult::Pass
    }

    fn record_trade(&self, pool: Pubkey, now_micros: u64) {
        let cutoff = now_micros.saturating_sub(ONE_HOUR_MICROS);

        // Per-asset: prune then push.
        {
            let mut entry = self.per_asset.entry(pool).or_default();
            entry.retain(|&ts| ts >= cutoff);
            entry.push_back(now_micros);
        }

        // Global: prune then push.
        {
            let mut g = self.global.lock().unwrap();
            g.retain(|&ts| ts >= cutoff);
            g.push_back(now_micros);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. MinEdgeFilter
// ─────────────────────────────────────────────────────────────────────────────

struct MinEdgeFilter {
    min_edge_bps: f64,
}

impl TradeFilter for MinEdgeFilter {
    fn name(&self) -> &'static str {
        "MinEdgeFilter"
    }

    fn check(
        &self,
        candidate: &TradeCandidate,
        _features: &ComputedFeatures,
        _now_micros: u64,
    ) -> FilterResult {
        if candidate.net_edge_bps < self.min_edge_bps {
            return FilterResult::Reject {
                reason: "net_edge_bps below min_edge_bps threshold",
            };
        }
        FilterResult::Pass
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TradeFilterChain
// ─────────────────────────────────────────────────────────────────────────────

/// Ordered pipeline of trade filters evaluated left-to-right.
///
/// Constructed once per `ScalpEngine`; filters are stored as trait objects so
/// new stages can be added without touching the engine.
pub struct TradeFilterChain {
    filters: Vec<Box<dyn TradeFilter>>,
}

impl TradeFilterChain {
    /// Constructs the default six-filter chain from `cfg`.
    pub fn new(cfg: &ScalerConfig) -> Self {
        Self {
            filters: vec![
                Box::new(LiquidityFilter {
                    min_tvl: cfg.min_pool_tvl_usd,
                    min_volume: cfg.min_volume_5m_usd,
                }),
                Box::new(VolatilityRegimeFilter {
                    floor: cfg.volatility_floor,
                    ceiling: cfg.volatility_ceiling,
                }),
                Box::new(SlippageFilter {
                    max_rt_cost_bps: cfg.max_slippage_bps,
                }),
                Box::new(CooldownFilter {
                    cooldown_micros: cfg.trade_cooldown_secs.saturating_mul(1_000_000),
                    last_trade: DashMap::new(),
                }),
                Box::new(RateLimitFilter {
                    max_per_asset: cfg.max_trades_per_hour_per_asset,
                    max_global: cfg.max_trades_per_hour_global,
                    per_asset: DashMap::new(),
                    global: Mutex::new(VecDeque::new()),
                }),
                Box::new(MinEdgeFilter {
                    min_edge_bps: cfg.min_edge_bps,
                }),
            ],
        }
    }

    /// Run every filter in order.
    ///
    /// Returns `None` when all filters pass, or `Some((filter_name, reason))`
    /// on the first rejection.
    pub fn check_all(
        &self,
        candidate: &TradeCandidate,
        features: &ComputedFeatures,
        now_micros: u64,
    ) -> Option<(&'static str, &'static str)> {
        for filter in &self.filters {
            match filter.check(candidate, features, now_micros) {
                FilterResult::Pass => {}
                FilterResult::Reject { reason } => {
                    return Some((filter.name(), reason));
                }
            }
        }
        None
    }

    /// Notify all stateful filters that a trade was executed.
    ///
    /// Must be called *after* a successful paper fill so that cooldown and
    /// rate-limit state reflect the execution.
    pub fn record_trade(&self, pool: Pubkey, now_micros: u64) {
        for filter in &self.filters {
            filter.record_trade(pool, now_micros);
        }
    }
}
