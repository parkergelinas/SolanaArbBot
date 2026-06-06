//! Shared market state containers.

use std::sync::atomic::{AtomicI64, Ordering};

use dashmap::DashMap;

use crate::contracts::{Candle, CandleInterval, TokenPrice};

#[derive(Clone, Debug)]
pub struct PoolSnapshot {
    pub pool_id: String,
    pub token_a: String,
    pub token_b: String,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub slot: u64,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct CandleKey {
    pub mint: String,
    pub interval: CandleInterval,
}

pub struct FlowTracker {
    buy_volume: AtomicI64,
    sell_volume: AtomicI64,
}

impl FlowTracker {
    pub fn new() -> Self {
        Self {
            buy_volume: AtomicI64::new(0),
            sell_volume: AtomicI64::new(0),
        }
    }

    pub fn record(&self, is_buy: bool, amount: u64) {
        let v = amount as i64;
        if is_buy {
            self.buy_volume.fetch_add(v, Ordering::Relaxed);
        } else {
            self.sell_volume.fetch_add(v, Ordering::Relaxed);
        }
    }

    pub fn imbalance(&self) -> f64 {
        let b = self.buy_volume.load(Ordering::Relaxed) as f64;
        let s = self.sell_volume.load(Ordering::Relaxed) as f64;
        let t = b + s;
        if t <= 0.0 {
            return 0.0;
        }
        (b - s) / t
    }
}

pub struct MarketState {
    pub prices: DashMap<String, TokenPrice>,
    pub pools: DashMap<String, PoolSnapshot>,
    pub candles: DashMap<CandleKey, Candle>,
    pub flow: DashMap<String, FlowTracker>,
}

/// Max tracked mints / pools — prevents unbounded growth on high-cardinality streams.
const MAX_TRACKED_KEYS: usize = 512;

fn trim_map<K: std::hash::Hash + Eq + Clone, V>(map: &DashMap<K, V>, max: usize) {
    let excess = map.len().saturating_sub(max);
    if excess == 0 {
        return;
    }
    let keys: Vec<K> = map.iter().take(excess).map(|e| e.key().clone()).collect();
    for k in keys {
        map.remove(&k);
    }
}

impl MarketState {
    pub fn new() -> Self {
        Self {
            prices: DashMap::new(),
            pools: DashMap::new(),
            candles: DashMap::new(),
            flow: DashMap::new(),
        }
    }

    /// Evict oldest entries when maps exceed [`MAX_TRACKED_KEYS`].
    pub fn enforce_bounds(&self) {
        trim_map(&self.prices, MAX_TRACKED_KEYS);
        trim_map(&self.pools, MAX_TRACKED_KEYS);
        trim_map(&self.flow, MAX_TRACKED_KEYS);
        trim_map(&self.candles, MAX_TRACKED_KEYS * 6);
    }
}

impl Default for MarketState {
    fn default() -> Self {
        Self::new()
    }
}
