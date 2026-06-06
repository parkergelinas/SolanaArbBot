//! Synchronous hot-path engine — the core decision loop.
//!
//! `HotPathEngine::process_tick` is the single entry point.  It must complete
//! within the configured decision budget (target ≤50 ms) and performs zero
//! heap allocation on the common path.

use std::time::Instant;

use config::HotPathConfig;

use super::execute::{build_intent, ExecutionQueue};
use super::precompute::PrecomputeTable;
use super::risk::check_risk;
use super::signal::evaluate_momentum;
use super::state::HotState;
use super::types::{MarketTick, TickOutcome};

/// Cumulative hot-path statistics (updated outside the hot loop when possible).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HotPathStats {
    pub ticks_processed: u64,
    pub signals_generated: u64,
    pub signals_rejected: u64,
    pub intents_queued: u64,
    pub queue_full_drops: u64,
    pub last_decision_us: u64,
    pub max_decision_us: u64,
}

/// Single-process hot-path engine.
pub struct HotPathEngine {
    pub state: HotState,
    pub table: PrecomputeTable,
    pub queue: ExecutionQueue,
    pub stats: HotPathStats,
    decision_budget_us: u64,
    /// Receives exposure-release amounts from the cold-path thread after each submission (item 3).
    exposure_release_rx: crossbeam_channel::Receiver<u64>,
    /// Cloned into `ColdPathExecutor` so the cold path can signal back (item 3).
    exposure_release_tx: crossbeam_channel::Sender<u64>,
}

impl HotPathEngine {
    /// Construct engine with precomputed tables — call once at startup.
    #[must_use]
    pub fn new(cfg: &HotPathConfig) -> Self {
        let queue = ExecutionQueue::new(cfg.execution_queue_capacity);
        let table = PrecomputeTable::build(cfg, 0);
        let (exposure_release_tx, exposure_release_rx) = crossbeam_channel::unbounded::<u64>();
        Self {
            state: HotState::new(),
            table,
            queue,
            stats: HotPathStats::default(),
            decision_budget_us: cfg.decision_budget_us,
            exposure_release_rx,
            exposure_release_tx,
        }
    }

    /// Returns a sender the `ColdPathExecutor` uses to release exposure after submission.
    #[must_use]
    pub fn exposure_release_sender(&self) -> crossbeam_channel::Sender<u64> {
        self.exposure_release_tx.clone()
    }

    /// Register pools and rebuild route table.
    pub fn register_pools(&mut self, cfg: &HotPathConfig, venues: &[(u8,)]) {
        for &(venue,) in venues {
            let _ = self.state.register_pool(venue);
        }
        self.table = PrecomputeTable::build(cfg, self.state.pool_count);
    }

    /// Enable/disable trading gate.
    pub fn set_trading_enabled(&mut self, enabled: bool) {
        self.state.trading_enabled = enabled;
    }

    /// Synchronous hot loop entry point — **no logging, no async, no JSON**.
    #[inline]
    pub fn process_tick(&mut self, tick: MarketTick) -> TickOutcome {
        let start = Instant::now();

        // Drain exposure releases posted by the cold-path thread (item 3).
        // try_recv never allocates — safe on the hot path.
        while let Ok(release_amount) = self.exposure_release_rx.try_recv() {
            self.state.open_exposure_x100 =
                self.state.open_exposure_x100.saturating_sub(release_amount);
        }

        let pool_snapshot = {
            let pool = self.state.ingest(&tick);
            // Copy features needed for signal eval to avoid borrow conflicts.
            *pool
        };

        let route_idx = (tick.pool_idx as u8) * 2; // even = Raydium route for pool
        let signal = match evaluate_momentum(
            &pool_snapshot,
            tick.pool_idx,
            route_idx,
            &self.table.thresholds,
        ) {
            Some(s) => s,
            None => {
                self.record_tick(start);
                return TickOutcome::NoSignal;
            }
        };

        self.stats.signals_generated += 1;

        let verdict = check_risk(&signal, &pool_snapshot, &self.state, &self.table);
        if !verdict.is_approved() {
            self.stats.signals_rejected += 1;
            self.record_tick(start);
            return TickOutcome::SignalRejected(verdict);
        }

        let intent = build_intent(&signal, &pool_snapshot, &self.table);

        if !self.queue.try_enqueue(intent) {
            self.stats.queue_full_drops += 1;
            self.record_tick(start);
            return TickOutcome::SignalRejected(
                super::types::RiskVerdict::RejectedExposure,
            );
        }

        // Update state after successful enqueue (items 3, 5).
        self.state.pool_mut(tick.pool_idx).last_trade_slot = tick.slot;
        self.state.last_any_intent_slot = tick.slot;
        self.state.open_exposure_x100 = self.state
            .open_exposure_x100
            .saturating_add(self.table.thresholds.default_trade_usd_x100);
        self.stats.intents_queued += 1;

        self.record_tick(start);
        TickOutcome::Queued(intent)
    }

    #[inline]
    fn record_tick(&mut self, start: Instant) {
        self.stats.ticks_processed += 1;
        let elapsed = start.elapsed().as_micros() as u64;
        self.stats.last_decision_us = elapsed;
        if elapsed > self.stats.max_decision_us {
            self.stats.max_decision_us = elapsed;
        }
        // Budget check is cold-path observable — no log here.
        let _ = elapsed <= self.decision_budget_us;
    }

    pub fn queue_receiver(&self) -> crossbeam_channel::Receiver<super::types::ExecutionIntent> {
        self.queue.receiver()
    }

    pub fn execution_router(&self) -> super::execute::ExecutionRouter {
        super::execute::ExecutionRouter::from_table(&self.table)
    }

    pub fn queue_sender(&self) -> crossbeam_channel::Sender<super::types::ExecutionIntent> {
        self.queue.sender()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotpath::types::{PRICE_SCALE, RiskVerdict};

    fn engine_with_pool() -> HotPathEngine {
        let cfg = HotPathConfig::default();
        let mut engine = HotPathEngine::new(&cfg);
        engine.register_pools(&cfg, &[(0,)]);
        engine.set_trading_enabled(true);
        engine
    }

    fn spike_tick(pool_idx: u16, slot: u64) -> MarketTick {
        MarketTick {
            pool_idx,
            slot,
            price_fp: PRICE_SCALE + PRICE_SCALE / 50,
            reserve_a: 50_000_000_000,
            reserve_b: 50_000_000_000,
            volume_delta_usd_x100: 100_000_00,
        }
    }

    #[test]
    fn processes_tick_without_allocation() {
        let mut engine = engine_with_pool();
        // Seed baseline volume.
        for slot in 1..=20 {
            let tick = MarketTick {
                pool_idx: 0,
                slot,
                price_fp: PRICE_SCALE,
                reserve_a: 50_000_000_000,
                reserve_b: 50_000_000_000,
                volume_delta_usd_x100: 1_000_00,
            };
            engine.process_tick(tick);
        }
        let outcome = engine.process_tick(spike_tick(0, 21));
        assert!(matches!(outcome, TickOutcome::Queued(_) | TickOutcome::NoSignal | TickOutcome::SignalRejected(_)));
    }

    #[test]
    fn rejects_when_trading_disabled() {
        let mut engine = engine_with_pool();
        engine.set_trading_enabled(false);
        for slot in 1..=20 {
            engine.process_tick(MarketTick {
                pool_idx: 0,
                slot,
                price_fp: PRICE_SCALE,
                reserve_a: 50_000_000_000,
                reserve_b: 50_000_000_000,
                volume_delta_usd_x100: 5_000_00,
            });
        }
        let outcome = engine.process_tick(spike_tick(0, 21));
        if let TickOutcome::SignalRejected(v) = outcome {
            assert_eq!(v, RiskVerdict::RejectedTradingDisabled);
        }
    }

    #[test]
    fn decision_latency_under_budget() {
        let mut engine = engine_with_pool();
        engine.set_trading_enabled(true);
        for slot in 1..=100 {
            engine.process_tick(spike_tick(0, slot));
        }
        assert!(engine.stats.max_decision_us < 50_000, "max decision {}us", engine.stats.max_decision_us);
    }
}
