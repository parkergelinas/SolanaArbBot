//! Signal aggregation, deduplication, cooldown enforcement, and confidence filtering.
//!
//! `SignalAggregator` is the final gate before signals are published to the bus.
//! All filtering decisions are driven by `SignalEngineConfig`; no hardcoded constants.
//!
//! # HOT PATH note
//! All methods take `&self` — DashMap handles sharded locking with no `Arc<RwLock<>>`.

use std::collections::HashSet;

use config::SignalEngineConfig;
use dashmap::DashMap;
use common::Pubkey;

use crate::types::SignalEvent;

/// Aggregates and filters signals from all processors.
///
/// State: per-pool last-emission timestamps for cooldown enforcement.
pub struct SignalAggregator {
    /// Pool → timestamp_micros of last emitted signal.
    last_emitted: DashMap<Pubkey, u64>,
}

impl SignalAggregator {
    pub fn new() -> Self {
        Self {
            last_emitted: DashMap::new(),
        }
    }

    /// Filters a batch of raw signals from all processors.
    ///
    /// The following rules are applied in order:
    ///
    /// 1. Reject signals below `signal_min_strength`.
    /// 2. Reject signals below `signal_min_confidence`.
    /// 3. Reject signals for pools still within their cooldown window.
    /// 4. Deduplicate: only the strongest signal per (pool, signal_type) pair passes.
    /// 5. Update cooldown timestamps for every signal that survives.
    pub fn filter(
        &self,
        signals: Vec<SignalEvent>,
        now_micros: u64,
        cfg: &SignalEngineConfig,
    ) -> Vec<SignalEvent> {
        let cooldown_micros = cfg.cooldown_secs.saturating_mul(1_000_000);

        // Phase 1 — strength / confidence gates.
        let candidates: Vec<SignalEvent> = signals
            .into_iter()
            .filter(|s| s.strength >= cfg.signal_min_strength)
            .filter(|s| s.confidence >= cfg.signal_min_confidence)
            .collect();

        // Phase 2 — cooldown gate.
        let after_cooldown: Vec<SignalEvent> = candidates
            .into_iter()
            .filter(|s| {
                self.last_emitted
                    .get(&s.pool_address)
                    .map_or(true, |ts| {
                        now_micros.saturating_sub(*ts) >= cooldown_micros
                    })
            })
            .collect();

        // Phase 3 — deduplicate: keep the strongest signal per (pool, type) pair.
        let mut best: std::collections::HashMap<(Pubkey, u8), SignalEvent> =
            std::collections::HashMap::new();

        for signal in after_cooldown {
            let key = (signal.pool_address, signal.signal_type as u8);
            let entry = best.entry(key).or_insert_with(|| signal.clone());
            if signal.strength > entry.strength {
                *entry = signal;
            }
        }

        let output: Vec<SignalEvent> = best.into_values().collect();

        // Phase 4 — update cooldown timestamps for emitted pools.
        let emitted_pools: HashSet<Pubkey> =
            output.iter().map(|s| s.pool_address).collect();
        for pool in emitted_pools {
            self.last_emitted.insert(pool, now_micros);
        }

        output
    }

    /// Returns the number of pools currently tracked in the cooldown map.
    pub fn tracked_pool_count(&self) -> usize {
        self.last_emitted.len()
    }

    /// Clears all cooldown state.  Intended for testing only.
    #[cfg(test)]
    pub fn reset(&self) {
        self.last_emitted.clear();
    }
}

impl Default for SignalAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Pubkey;
    use config::SignalEngineConfig;
    use crate::types::{Direction, FeatureVector, SignalEvent, SignalType};

    fn pool(b: u8) -> Pubkey { Pubkey::new([b; 32]) }

    fn cfg() -> SignalEngineConfig {
        SignalEngineConfig {
            signal_min_strength: 0.30,
            signal_min_confidence: 0.40,
            cooldown_secs: 10,
            ..SignalEngineConfig::default()
        }
    }

    fn make_signal(pool: Pubkey, strength: f64, confidence: f64) -> SignalEvent {
        SignalEvent {
            signal_id: 1,
            timestamp_micros: 1_000_000,
            pool_address: pool,
            signal_type: SignalType::Momentum,
            strength,
            confidence,
            direction: Direction::Long,
            timeframe_secs: 60,
            feature_vector: FeatureVector {
                volume_short: 1.0,
                volume_long: 2.0,
                price_velocity: 0.01,
                liquidity_delta_pct: 0.0,
                whale_activity_score: 0.0,
                smart_money_score: 0.0,
                data_points: 10,
            },
            explanation: "test".to_owned(),
        }
    }

    #[test]
    fn low_strength_rejected() {
        let agg = SignalAggregator::new();
        let s = make_signal(pool(1), 0.10, 0.80); // strength < 0.30
        let out = agg.filter(vec![s], 1_000_000, &cfg());
        assert!(out.is_empty());
    }

    #[test]
    fn low_confidence_rejected() {
        let agg = SignalAggregator::new();
        let s = make_signal(pool(1), 0.80, 0.20); // confidence < 0.40
        let out = agg.filter(vec![s], 1_000_000, &cfg());
        assert!(out.is_empty());
    }

    #[test]
    fn valid_signal_passes() {
        let agg = SignalAggregator::new();
        let s = make_signal(pool(1), 0.70, 0.70);
        let out = agg.filter(vec![s], 1_000_000, &cfg());
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn cooldown_blocks_immediate_reemission() {
        let agg = SignalAggregator::new();
        let p = pool(2);
        let now = 50_000_000_u64;

        // First emission.
        let first = agg.filter(vec![make_signal(p, 0.7, 0.7)], now, &cfg());
        assert_eq!(first.len(), 1);

        // Same pool 1 s later — still in 10 s cooldown.
        let second = agg.filter(vec![make_signal(p, 0.7, 0.7)], now + 1_000_000, &cfg());
        assert!(second.is_empty(), "should be suppressed by cooldown");
    }

    #[test]
    fn cooldown_allows_emission_after_expiry() {
        let agg = SignalAggregator::new();
        let p = pool(3);
        let now = 100_000_000_u64;

        agg.filter(vec![make_signal(p, 0.7, 0.7)], now, &cfg());

        // 11 s later — cooldown expired.
        let second =
            agg.filter(vec![make_signal(p, 0.7, 0.7)], now + 11_000_000, &cfg());
        assert_eq!(second.len(), 1);
    }

    #[test]
    fn deduplication_keeps_strongest_signal() {
        let agg = SignalAggregator::new();
        let p = pool(4);
        let weak = make_signal(p, 0.4, 0.6);
        let strong = make_signal(p, 0.9, 0.8);
        let out = agg.filter(vec![weak, strong], 1_000_000, &cfg());
        assert_eq!(out.len(), 1);
        assert!((out[0].strength - 0.9).abs() < 1e-9);
    }

    #[test]
    fn different_pools_both_pass() {
        let agg = SignalAggregator::new();
        let signals = vec![
            make_signal(pool(5), 0.7, 0.7),
            make_signal(pool(6), 0.8, 0.8),
        ];
        let out = agg.filter(signals, 1_000_000, &cfg());
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn empty_input_returns_empty() {
        let agg = SignalAggregator::new();
        let out = agg.filter(vec![], 1_000_000, &cfg());
        assert!(out.is_empty());
    }
}
