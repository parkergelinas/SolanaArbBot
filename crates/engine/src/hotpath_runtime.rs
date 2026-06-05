//! Hot-path runtime — wires ingestion, hot loop, and cold-path I/O threads.
//!
//! Single-process architecture:
//! ```text
//! [Ingestion Thread] --SPSC--> [Hot Loop Thread] --SPSC--> [Cold I/O Thread]
//! ```

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use config::{ConfigHandle, HotPathConfig};
use crossbeam_channel::{bounded, Sender};
use execution::hotpath::{ColdPathExecutor, HotPathEngine, MarketTick};
use tracing::info;

/// Inbound market tick channel capacity.
const TICK_CHANNEL_CAPACITY: usize = 4096;

/// Hot-path runtime handle.
pub struct HotPathRuntime {
    tick_tx: Sender<MarketTick>,
    engine: Arc<std::sync::Mutex<HotPathEngine>>,
}

impl HotPathRuntime {
    /// Bootstrap the full single-process hot-path system.
    pub fn start(cfg: &HotPathConfig) -> Self {
        let mut engine = HotPathEngine::new(cfg);
        engine.register_pools(
            cfg,
            &[(0,), (1,), (0,), (1,), (0,)], // 5 demo pools: Raydium/Orca mix
        );
        engine.set_trading_enabled(true);

        let (tick_tx, tick_rx) = bounded(TICK_CHANNEL_CAPACITY);
        let queue_rx = engine.queue_receiver();
        let router = engine.execution_router();

        let engine = Arc::new(std::sync::Mutex::new(engine));

        // ── Cold-path I/O thread (Jito / RPC) ────────────────────────────────
        let cold = ColdPathExecutor::new(router, queue_rx);
        thread::Builder::new()
            .name("cold-io".into())
            .spawn(move || cold.run_loop())
            .expect("spawn cold-io");

        // ── Hot loop thread (synchronous, no async) ────────────────────────
        let hot_engine = Arc::clone(&engine);
        thread::Builder::new()
            .name("hot-loop".into())
            .spawn(move || {
                while let Ok(tick) = tick_rx.recv() {
                    let mut eng = hot_engine.lock().expect("hot engine lock");
                    let _outcome = eng.process_tick(tick);
                    // No logging in hot loop — stats read from cold/monitor thread.
                }
            })
            .expect("spawn hot-loop");

        Self { tick_tx, engine }
    }

    /// Load config from disk and start runtime.
    pub fn from_config() -> Self {
        let handle = ConfigHandle::load().expect("config load");
        Self::start(&handle.hotpath)
    }

    /// Publish a market tick into the hot loop (from ingestion thread).
    #[inline]
    pub fn publish_tick(&self, tick: MarketTick) {
        let _ = self.tick_tx.try_send(tick);
    }

    /// Returns a clone of the tick sender for ingestion producers.
    pub fn tick_sender(&self) -> Sender<MarketTick> {
        self.tick_tx.clone()
    }

    /// Read hot-path stats (cold path — safe to call from monitor thread).
    pub fn stats(&self) -> execution::hotpath::HotPathStats {
        self.engine
            .lock()
            .expect("hot engine lock")
            .stats
    }

    /// Enable or disable trading.
    pub fn set_trading_enabled(&self, enabled: bool) {
        self.engine
            .lock()
            .expect("hot engine lock")
            .set_trading_enabled(enabled);
    }
}

/// Synthetic ingestion producer for demo / paper mode.
pub fn spawn_synthetic_ingestion(tx: Sender<MarketTick>, pool_count: u16) {
    thread::Builder::new()
        .name("ingestion".into())
        .spawn(move || {
            let mut slot: u64 = 1;
            let mut rng_state: u64 = 0xDEAD_BEEF_CAFE_BABE;
            loop {
                // xorshift64 — deterministic, no rand crate needed.
                rng_state ^= rng_state << 13;
                rng_state ^= rng_state >> 7;
                rng_state ^= rng_state << 17;

                let pool_idx = (rng_state % pool_count as u64) as u16;
                let price_jitter = (rng_state % 200) as u64;
                let volume = ((rng_state >> 8) % 500_000) as u64 + 10_000;

                let tick = MarketTick {
                    pool_idx,
                    slot,
                    price_fp: 1_000_000_000 + price_jitter * 1_000_000,
                    reserve_a: 50_000_000_000,
                    reserve_b: 50_000_000_000,
                    volume_delta_usd_x100: volume,
                };

                let _ = tx.try_send(tick);
                slot += 1;

                // 250–500 ms between ticks.
                let delay_ms = 250 + (rng_state % 250) as u64;
                thread::sleep(Duration::from_millis(delay_ms));
            }
        })
        .expect("spawn ingestion");
}

/// Monitor thread — logs stats every 5 s (outside hot loop).
pub fn spawn_monitor(runtime: Arc<HotPathRuntime>) {
    thread::Builder::new()
        .name("monitor".into())
        .spawn(move || loop {
            thread::sleep(Duration::from_secs(5));
            let stats = runtime.stats();
            info!(
                ticks = stats.ticks_processed,
                signals = stats.signals_generated,
                queued = stats.intents_queued,
                rejected = stats.signals_rejected,
                last_us = stats.last_decision_us,
                max_us = stats.max_decision_us,
                "hot-path stats"
            );
        })
        .expect("spawn monitor");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_starts_and_processes_ticks() {
        let cfg = HotPathConfig::default();
        let runtime = HotPathRuntime::start(&cfg);
        let tx = runtime.tick_sender();

        for slot in 1..=50 {
            tx.send(MarketTick {
                pool_idx: 0,
                slot,
                price_fp: 1_000_000_000 + slot * 1_000_000,
                reserve_a: 50_000_000_000,
                reserve_b: 50_000_000_000,
                volume_delta_usd_x100: 50_000_00,
            })
            .expect("send tick");
        }

        thread::sleep(Duration::from_millis(200));
        assert!(runtime.stats().ticks_processed > 0);
    }
}
