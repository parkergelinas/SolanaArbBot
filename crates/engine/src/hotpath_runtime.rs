//! Hot-path runtime — wires ingestion, hot loop, and cold-path I/O threads.
//!
//! Single-process architecture:
//! ```text
//! [Ingestion Thread] --SPSC--> [Hot Loop Thread] --SPSC--> [Cold I/O Thread]
//! ```

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use config::{ConfigHandle, HotPathConfig, SystemConfig};
use crossbeam_channel::{bounded, Sender};
use execution::hotpath::{ColdPathExecutor, HotPathEngine, MarketTick};
use tokio::sync::Mutex;
use tracing::info;
use wallet::WalletKeypair;

use crate::arbitrage::{ArbitrageEngine, DexVenue, PoolQuote};

/// Inbound market tick channel capacity.
const TICK_CHANNEL_CAPACITY: usize = 4096;

/// Hot-path runtime handle.
pub struct HotPathRuntime {
    tick_tx: Sender<MarketTick>,
    engine: Arc<std::sync::Mutex<HotPathEngine>>,
}

impl HotPathRuntime {
    /// Bootstrap the full single-process hot-path system.
    ///
    /// Attempts to load a live keypair from `SOLANA_ARB_WALLET_KEY` when
    /// `cfg.paper_mode` is false.  Falls back to `None` (paper mode) if the
    /// env var is missing or the system config has `dry_run = true`.
    pub fn start(cfg: &HotPathConfig) -> Self {
        let system_cfg = SystemConfig::default();
        Self::start_with_config(cfg, &system_cfg)
    }

    /// Bootstrap with an explicit `SystemConfig` (used by the main binary).
    pub fn start_with_config(cfg: &HotPathConfig, sys: &SystemConfig) -> Self {
        let mut engine = HotPathEngine::new(cfg);
        engine.register_pools(
            cfg,
            &[(0,), (1,), (0,), (1,), (0,)], // 5 demo pools: Raydium/Orca mix
        );
        engine.set_trading_enabled(true);

        let (tick_tx, tick_rx) = bounded(TICK_CHANNEL_CAPACITY);
        let queue_rx = engine.queue_receiver();
        let router = engine.execution_router();
        let exposure_tx = engine.exposure_release_sender();

        let engine = Arc::new(std::sync::Mutex::new(engine));

        // ── Wallet (optional — bot runs without one in paper mode) ───────────
        let wallet: Option<Arc<WalletKeypair>> = match WalletKeypair::load_wallet_key(sys) {
            Ok(kp) => {
                info!("hot-path: live wallet loaded");
                Some(Arc::new(kp))
            }
            Err(e) => {
                info!(reason = %e, "hot-path: no wallet loaded — running without signing (paper mode)");
                None
            }
        };

        let rpc_endpoint = sys.rpc.primary_endpoint().to_owned();

        // ── Cold-path I/O thread (Jito / RPC) ────────────────────────────────
        let cold = ColdPathExecutor::new(router, queue_rx, exposure_tx, rpc_endpoint, wallet);
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
        Self::start_with_config(&handle.hotpath, &*handle)
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

/// Spawns the cross-DEX arbitrage detection loop (async, outside hot path).
///
/// Respects `features.dry_run` and `strategy.arb` — logs opportunities in paper
/// mode without submitting Jito bundles.
pub fn spawn_arb_engine(config: Arc<SystemConfig>) -> tokio::task::JoinHandle<()> {
    let sol_price_usd: f64 = std::env::var("SOLANA_ARB_SOL_PRICE_USD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(150.0);

    let engine = ArbitrageEngine::new(
        config.arbitrage.clone(),
        config.execution.clone(),
        config.features.clone(),
        sol_price_usd,
    );

    // Seed demo pools for paper/dry-run detection.
    let mut engine = engine;
    engine.upsert_pool(
        "ray-sol-usdc",
        PoolQuote {
            venue: DexVenue::RaydiumAmmV4,
            token_a: "SOL".into(),
            token_b: "USDC".into(),
            mid_price: 150.0,
            reserve_a: 50_000.0,
            reserve_b: 7_500_000.0,
            liquidity_usd: 80_000.0,
            fee_bps: 25,
        },
    );
    engine.upsert_pool(
        "orca-sol-usdc",
        PoolQuote {
            venue: DexVenue::OrcaWhirlpool,
            token_a: "SOL".into(),
            token_b: "USDC".into(),
            mid_price: 151.5,
            reserve_a: 50_000.0,
            reserve_b: 7_575_000.0,
            liquidity_usd: 80_000.0,
            fee_bps: 20,
        },
    );

    let shared = Arc::new(Mutex::new(engine));

    tokio::spawn(async move {
        let interval = std::time::Duration::from_millis(100);
        loop {
            let report = {
                let mut eng = shared.lock().await;
                eng.run_detection_cycle().await
            };
            if report.opportunities_emitted > 0 {
                info!(
                    found = report.opportunities_found,
                    emitted = report.opportunities_emitted,
                    submitted = report.bundles_submitted,
                    latency_us = report.cycle_latency_us,
                    dry_run = config.features.dry_run,
                    "arb engine cycle"
                );
            }
            tokio::time::sleep(interval).await;
        }
    })
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
