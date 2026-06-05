//! Lock-free pool state engine — DashMap keyed by (pair, dex).

use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;

use crate::types::PoolPrice;

const EMA_ALPHA: f64 = 0.2;

#[derive(Clone, Debug)]
pub struct PoolEntry {
    pub price: f64,
    pub liquidity: f64,
    pub timestamp: u64,
    pub volatility_ema: f64,
}

pub struct PoolStateEngine {
    pools: DashMap<(String, String), PoolEntry>,
    updates: AtomicU64,
}

impl PoolStateEngine {
    pub fn new() -> Self {
        Self {
            pools: DashMap::new(),
            updates: AtomicU64::new(0),
        }
    }

    pub fn upsert(&self, price: &PoolPrice) {
        let key = price.pool_key();
        let mut volatility_ema = 0.0;

        if let Some(mut existing) = self.pools.get_mut(&key) {
            let prev = existing.price;
            if prev > 0.0 {
                let change = ((price.price - prev) / prev).abs();
                volatility_ema = EMA_ALPHA * change + (1.0 - EMA_ALPHA) * existing.volatility_ema;
            } else {
                volatility_ema = existing.volatility_ema;
            }
            existing.price = price.price;
            existing.liquidity = price.liquidity;
            existing.timestamp = price.timestamp;
            existing.volatility_ema = volatility_ema;
        } else {
            self.pools.insert(
                key,
                PoolEntry {
                    price: price.price,
                    liquidity: price.liquidity,
                    timestamp: price.timestamp,
                    volatility_ema: 0.0,
                },
            );
        }
        self.updates.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self, pair: &str, dex: &str) -> Option<PoolEntry> {
        self.pools.get(&(pair.to_string(), dex.to_string())).map(|e| e.clone())
    }

    pub fn pairs(&self) -> Vec<String> {
        let mut pairs: Vec<String> = self
            .pools
            .iter()
            .map(|e| e.key().0.clone())
            .collect();
        pairs.sort();
        pairs.dedup();
        pairs
    }

    pub fn venues_for_pair(&self, pair: &str) -> Vec<(String, PoolEntry)> {
        self.pools
            .iter()
            .filter(|e| e.key().0 == pair)
            .map(|e| (e.key().1.clone(), e.value().clone()))
            .collect()
    }

    pub fn update_count(&self) -> u64 {
        self.updates.load(Ordering::Relaxed)
    }
}

impl Default for PoolStateEngine {
    fn default() -> Self {
        Self::new()
    }
}
