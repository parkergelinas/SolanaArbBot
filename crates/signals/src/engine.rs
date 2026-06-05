//! Top-level signal engine coordinator.
//!
//! # Strict event flow (from `.cursor/workflow.md`)
//!
//! ```text
//! SignalInput
//!   │
//!   ▼
//! FeatureStore::update_*   ← update rolling windows
//!   │
//!   ▼
//! SignalProcessors          ← run all processors in order
//!   │
//!   ▼
//! SignalAggregator::filter  ← dedup / cooldown / threshold gate
//!   │
//!   ▼
//! SignalSender (crossbeam)  ← publish surviving signals
//! ```
//!
//! # Thread-safety
//!
//! `SignalEngine` is `Send + Sync`.  All internal types use `DashMap` for
//! sharded locking — no `Arc<RwLock<>>` is present anywhere.
//!
//! # Determinism
//!
//! `process_at(input, now_micros)` is deterministic for identical
//! `(input, store_state, config, now_micros)` tuples.  `process(input)` calls
//! it with the current wall clock.

use config::SignalEngineConfig;
use tracing::{debug, warn};

use crate::aggregator::SignalAggregator;
use crate::bus::{SignalSender, signal_channel, SignalReceiver};
use crate::feature_store::FeatureStore;
use crate::processor::SignalProcessor;
use crate::processors::{MomentumProcessor, SmartMoneyProcessor, WhaleFlowProcessor};
use crate::types::{SignalEvent, SignalInput, unix_micros};

/// Assembled signal engine.
///
/// Construct once at startup via [`SignalEngine::new`], share as
/// `Arc<SignalEngine>` if multi-producer access is required.
pub struct SignalEngine {
    feature_store: FeatureStore,
    processors: Vec<Box<dyn SignalProcessor>>,
    aggregator: SignalAggregator,
    config: SignalEngineConfig,
    sender: SignalSender,
}

impl SignalEngine {
    /// Builds the engine with the standard processor set and returns
    /// `(engine, receiver)`.  The caller owns the receiver and can pass it
    /// to any downstream consumer (paper trading, risk engine, etc.).
    pub fn new(cfg: SignalEngineConfig) -> (Self, SignalReceiver) {
        let (sender, receiver) = signal_channel(cfg.signal_channel_capacity);

        let engine = Self {
            feature_store: FeatureStore::new(cfg.feature_store_max_age_secs),
            processors: vec![
                Box::new(WhaleFlowProcessor),
                Box::new(SmartMoneyProcessor),
                Box::new(MomentumProcessor),
            ],
            aggregator: SignalAggregator::new(),
            config: cfg,
            sender,
        };

        (engine, receiver)
    }

    /// Processes `input` using the current wall clock.
    ///
    /// Returned `Vec<SignalEvent>` mirrors what was published to the bus
    /// (useful for testing without a receiver).
    pub fn process(&self, input: SignalInput) -> Vec<SignalEvent> {
        self.process_at(input, unix_micros())
    }

    /// Deterministic variant: processes `input` as if the current time is
    /// `now_micros`.  Use this in tests for reproducibility.
    pub fn process_at(&self, input: SignalInput, now_micros: u64) -> Vec<SignalEvent> {
        // Step 1 — update feature store.
        match &input {
            SignalInput::Market(me) => self.feature_store.update_market(me, now_micros),
            SignalInput::Whale(we) => self.feature_store.update_whale(we),
        }

        // Step 2 — determine pool; bail early if unknown.
        let pool = match input.pool_address() {
            Some(p) => p,
            None => {
                debug!("SignalEngine: ignoring event with no pool address");
                return vec![];
            }
        };

        // Step 3 — compute features once for all processors.
        let features = self.feature_store.compute(pool, now_micros, &self.config);

        // Step 4 — run processors, collect raw signals.
        let raw: Vec<SignalEvent> = self
            .processors
            .iter()
            .flat_map(|p| {
                let signals = p.process(&input, &features, &self.config, now_micros);
                if !signals.is_empty() {
                    debug!(
                        processor = p.name(),
                        count = signals.len(),
                        "raw signals produced"
                    );
                }
                signals
            })
            .collect();

        // Step 5 — aggregate (dedup / cooldown / threshold).
        let output = self.aggregator.filter(raw, now_micros, &self.config);

        // Step 6 — publish to bus.
        for signal in &output {
            if let Err(e) = self.sender.try_send(signal.clone()) {
                warn!(
                    pool = %format!("{:?}", signal.pool_address),
                    signal_type = %signal.signal_type,
                    "signal bus full, dropping signal: {e}"
                );
            }
        }

        output
    }

    /// Read-only access to the underlying feature store (e.g. for diagnostics).
    pub fn feature_store(&self) -> &FeatureStore {
        &self.feature_store
    }

    /// Read-only access to the engine configuration.
    pub fn config(&self) -> &SignalEngineConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{MarketEvent, Pubkey, SwapEvent, PoolUpdate};
    use config::SignalEngineConfig;
    use crate::types::{Direction, WhaleEvent};

    fn pool() -> Pubkey { Pubkey::new([42; 32]) }

    fn test_cfg() -> SignalEngineConfig {
        SignalEngineConfig {
            signal_min_strength: 0.10,
            signal_min_confidence: 0.10,
            cooldown_secs: 5,
            whale_threshold_usd: 10_000.0,
            smart_money_min_score: 0.70,
            momentum_window_secs: 60,
            feature_store_max_age_secs: 300,
            signal_channel_capacity: 64,
        }
    }

    #[test]
    fn pool_update_without_pool_returns_empty() {
        let (eng, _rx) = SignalEngine::new(test_cfg());
        let evt = MarketEvent::PoolUpdate(PoolUpdate {
            pool: None,
            token_a_mint: None,
            token_b_mint: None,
            liquidity: Some(1_000),
            sqrt_price: Some(2_000),
            fee_rate: None,
        });
        let out = eng.process_at(SignalInput::Market(evt), 1_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn below_threshold_whale_emits_nothing() {
        let (eng, _rx) = SignalEngine::new(test_cfg());
        let p = pool();
        let we = WhaleEvent {
            timestamp_micros: 1_000_000,
            pool_address: p,
            swap_amount_usd: 100.0, // far below 10_000 threshold
            direction: Direction::Long,
            profitability_score: 0.5,
        };
        let out = eng.process_at(SignalInput::Whale(we), 2_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn qualifying_whale_reaches_bus() {
        let (eng, rx) = SignalEngine::new(test_cfg());
        let p = pool();

        // Seed the feature store with some history.
        for i in 0u64..5 {
            let evt = MarketEvent::SwapEvent(SwapEvent {
                pool: p,
                input_mint: Pubkey::new([0; 32]),
                output_mint: Pubkey::new([0; 32]),
                amount_in: 1_000,
                amount_out: 998,
            });
            eng.process_at(SignalInput::Market(evt), i * 5_000_000);
        }

        let we = WhaleEvent {
            timestamp_micros: 25_000_000,
            pool_address: p,
            swap_amount_usd: 50_000.0,
            direction: Direction::Long,
            profitability_score: 0.8,
        };
        let out = eng.process_at(SignalInput::Whale(we), 25_000_000);
        // At least a WhaleFlow signal should be emitted.
        assert!(!out.is_empty(), "expected at least one signal");
        assert!(rx.try_recv().is_ok(), "signal should be on the bus");
    }

    #[test]
    fn engine_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SignalEngine>();
    }
}
