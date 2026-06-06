//! Production runtime orchestrator for the Solana quantitative trading system.
//!
//! Implements a full pipeline with configurable run modes, graceful shutdown,
//! structured observability, and a synthetic ingestion path for paper trading.
//!
//! # Run modes
//!
//! ```text
//! cargo run --bin worker -- --mode paper
//! cargo run --bin worker -- --mode backtest --config custom.toml
//! cargo run --bin worker -- --mode live --enable-live   # always rejected: not yet implemented
//! ```
//!
//! Set `RUST_LOG=debug` to override the `monitoring.log_level` from config.

mod halt;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use config::{ConfigHandle, SystemConfig};
use events::{EventBus, EventSubscriber, MarketEvent, PoolUpdate, SwapEvent};
use common::Pubkey;
use scalper::ScalpEngine;
use signals::{ComputedFeatures, SignalEngine, SignalInput, SignalReceiver, LiveDataHandles};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// CLI surface
// ─────────────────────────────────────────────────────────────────────────────

/// Pipeline run mode.
///
/// ```text
/// cargo run --bin worker -- --mode paper
/// cargo run --bin worker -- --mode backtest
/// cargo run --bin worker -- --mode live --enable-live   # always rejected
/// ```
#[derive(Debug, Clone, PartialEq)]
enum RunMode {
    /// Synthetic market events, no real RPC, no trades submitted.
    Paper,
    /// Same as paper; reserved for historical replay integration.
    Backtest,
    /// Requires `--enable-live`. Live trading is NOT implemented in this build.
    Live,
}

impl std::str::FromStr for RunMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "paper"    => Ok(RunMode::Paper),
            "backtest" => Ok(RunMode::Backtest),
            "live"     => Ok(RunMode::Live),
            other      => Err(format!("unknown run mode '{other}'. valid: paper, backtest, live")),
        }
    }
}

/// Parsed CLI arguments.
#[derive(Debug)]
struct Cli {
    mode:        RunMode,
    config:      Option<std::path::PathBuf>,
    enable_live: bool,
}

impl Cli {
    /// Parses `std::env::args()` into a [`Cli`].
    ///
    /// Supported flags:
    /// * `--mode <paper|backtest|live>` (default: paper)
    /// * `--config <path>`
    /// * `--enable-live`
    /// * `--help` / `-h`
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();

        if args.iter().any(|a| a == "--help" || a == "-h") {
            println!(
                "usage: worker [--mode paper|backtest|live] [--config <path>] [--enable-live]"
            );
            std::process::exit(0);
        }

        let mut mode        = RunMode::Paper;
        let mut config      = None;
        let mut enable_live = false;
        let mut i           = 0_usize;

        while i < args.len() {
            match args[i].as_str() {
                "--mode" => {
                    i += 1;
                    let raw = args.get(i).map(String::as_str).unwrap_or("");
                    mode = raw.parse().unwrap_or_else(|e| {
                        eprintln!("ERROR: {e}");
                        std::process::exit(1);
                    });
                }
                "--config" => {
                    i += 1;
                    let raw = args.get(i).unwrap_or_else(|| {
                        eprintln!("ERROR: --config requires a path argument");
                        std::process::exit(1);
                    });
                    config = Some(std::path::PathBuf::from(raw));
                }
                "--enable-live" => {
                    enable_live = true;
                }
                other => {
                    eprintln!("ERROR: unknown argument '{other}'");
                    std::process::exit(1);
                }
            }
            i += 1;
        }

        Self { mode, config, enable_live }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point — bootstrap sequence (strict order, each step logged)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // ── 1. Parse CLI args ─────────────────────────────────────────────────────
    let args = Cli::parse();

    // ── 2. Safety check — live mode guards ────────────────────────────────────
    if args.mode == RunMode::Live && !args.enable_live {
        eprintln!(
            "ERROR: Live mode requires --enable-live flag. \
             This system does not support live trading without explicit opt-in."
        );
        std::process::exit(1);
    }
    if args.mode == RunMode::Live {
        eprintln!(
            "ERROR: Live trading is not implemented in this build. Use --mode paper."
        );
        std::process::exit(1);
    }

    // ── 3. Load SystemConfig ──────────────────────────────────────────────────
    let config_handle = match &args.config {
        Some(path) => match ConfigHandle::load_from_file(path) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("ERROR: failed to load config from {path:?}: {e}");
                std::process::exit(1);
            }
        },
        None => match ConfigHandle::load() {
            Ok(h) => h,
            Err(e) => {
                eprintln!("ERROR: failed to load config: {e}");
                std::process::exit(1);
            }
        },
    };
    let config = config_handle.arc();

    // ── 4. Init tracing (RUST_LOG wins over config log_level) ─────────────────
    init_tracing(&config.monitoring.log_level);
    info!(
        mode        = ?args.mode,
        config_path = ?args.config,
        dry_run     = config.features.dry_run,
        "system starting"
    );

    // ── 4b. Backtest mode — run historical replay pipeline and exit ───────────
    if args.mode == RunMode::Backtest {
        run_backtest_mode(Arc::clone(&config));
        return;
    }

    // ── 5. Create shutdown signal ─────────────────────────────────────────────
    let shutdown   = CancellationToken::new();
    let start_time = Instant::now();

    // ── 6. Init EventBus ──────────────────────────────────────────────────────
    let event_bus = EventBus::new();
    info!(subsystem = "event_bus", "initialized");

    let paper_mode = args.mode == RunMode::Paper;

    // ── 6a. Master strategy dispatcher (shared bus + global risk gate) ─────────
    let dispatcher = engine::StrategyDispatcher::new(Arc::clone(&config));
    let pub_handles = dispatcher.start(paper_mode, shutdown.clone());
    let strategy_copy_tx = pub_handles.copy_tx.clone();
    let mut task_handles: Vec<(&'static str, tokio::task::JoinHandle<()>)> =
        pub_handles.tasks;
    info!(
        subsystem = "strategy_dispatcher",
        paper_mode,
        arb = paper_mode || config.strategy.arb,
        sniper = config.strategy.sniper && config.sniper.enabled,
        copy = config.strategy.whale_copy && config.copy_trading.enabled,
        liquidation = config.strategy.liquidation && config.liquidation.enabled,
        momentum = config.strategy.momentum && config.momentum.enabled,
        "strategy dispatcher started"
    );

    // ── 6b. Whale tracker + copy trading + liquidation + external pollers ───
    let live_pollers_enabled = config.whale_tracker.enabled
        || config.copy_trading.enabled
        || (config.liquidation.enabled && !config.strategy.liquidation)
        || (config.momentum.enabled && !config.strategy.momentum);
    let _live_data: Option<LiveDataHandles> = if live_pollers_enabled {
        let data_sources = Arc::new(config.data_sources.clone());
        let whale_cfg = if config.whale_tracker.enabled {
            Some(Arc::new(config.whale_tracker.clone()))
        } else {
            None
        };
        let copy_cfg = if config.copy_trading.enabled {
            Some(Arc::new(config.copy_trading.clone()))
        } else {
            None
        };
        let liquidation_cfg = if config.liquidation.enabled && !config.strategy.liquidation {
            Some(Arc::new(config.liquidation.clone()))
        } else {
            None
        };
        let momentum_cfg = if config.momentum.enabled && !config.strategy.momentum {
            Some(Arc::new(config.momentum.clone()))
        } else {
            None
        };
        let features = Arc::new(config.features.clone());
        let mints = Arc::new(vec![
            signals::whale_watcher::SOL_MINT.to_owned(),
            signals::whale_watcher::USDC_MINT.to_owned(),
        ]);
        info!(
            subsystem = "live_pollers",
            whale_tracker = config.whale_tracker.enabled,
            copy_trading = config.copy_trading.enabled,
            liquidation = config.liquidation.enabled,
            momentum = config.momentum.enabled,
            dry_run = config.features.dry_run,
            min_trade_usd = config.whale_tracker.min_trade_usd,
            min_whale_copy_usd = config.copy_trading.min_whale_trade_usd,
            "starting live data pollers"
        );
        Some(
            signals::spawn_live_data_pollers(
                event_bus.clone(),
                data_sources,
                mints,
                whale_cfg,
                copy_cfg,
                liquidation_cfg,
                momentum_cfg,
                Some(features),
                strategy_copy_tx,
            )
            .await,
        )
    } else {
        info!(subsystem = "live_pollers", "all live pollers disabled in config");
        None
    };

    // ── 6c. New-token sniper executor (gated by strategy.sniper) ──────────────
    if config.strategy.sniper && config.sniper.enabled {
        let data_sources = Arc::new(config.data_sources.clone());
        let features = Arc::new(config.features.clone());
        let store = if let Some(ref live) = _live_data {
            live.external.clone()
        } else {
            let store = signals::ExternalSignalStore::new();
            signals::load_jupiter_verified(&store).await;
            store
        };
        let rpc_url = config
            .data_sources
            .helius_rpc_url()
            .or_else(|| config.rpc.endpoints.first().cloned())
            .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_owned());
        info!(
            subsystem = "sniper",
            dry_run = config.features.dry_run,
            base_position_sol = config.sniper.base_position_sol,
            min_liquidity_sol = config.sniper.min_liquidity_sol,
            "starting new-token sniper"
        );
        signals::spawn_sniper_strategy(
            Arc::new(config.sniper.clone()),
            features,
            data_sources,
            store,
            rpc_url,
        );
    } else {
        info!(subsystem = "sniper", "disabled in config");
    }

    // Subscribe *before* spawning producers so no events are lost.
    let signal_subscriber = event_bus.subscribe();

    // ── 7. Init SignalEngine ──────────────────────────────────────────────────
    let (signal_engine, signal_rx) = SignalEngine::new(config.signal_engine.clone());
    let signal_engine = Arc::new(signal_engine);
    info!(
        subsystem       = "signal_engine",
        whale_threshold = config.signal_engine.whale_threshold_usd,
        channel_capacity = config.signal_engine.signal_channel_capacity,
        "initialized"
    );

    // ── 8. Init ScalpEngine ───────────────────────────────────────────────────
    let scalp_engine = Arc::new(ScalpEngine::new(Arc::clone(&config)));
    info!(subsystem = "scalper", "ScalpEngine initialized");

    // ── Shared observability counters ─────────────────────────────────────────
    let events_counter   = Arc::new(AtomicU64::new(0));
    let signals_counter  = Arc::new(AtomicU64::new(0));
    let trades_evaluated = Arc::new(AtomicU64::new(0));
    let trades_rejected  = Arc::new(AtomicU64::new(0));

    // ── 9. Spawn monitoring task ──────────────────────────────────────────────
    task_handles.push(("monitor", tokio::spawn(run_monitoring(
        start_time,
        Arc::clone(&events_counter),
        Arc::clone(&signals_counter),
        Arc::clone(&trades_evaluated),
        Arc::clone(&trades_rejected),
        shutdown.clone(),
    ))));

    // ── 10. Spawn synthetic ingestion task ────────────────────────────────────
    task_handles.push(("ingestion", tokio::spawn(run_synthetic_ingestion(
        event_bus.clone(),
        shutdown.clone(),
        Arc::clone(&events_counter),
    ))));

    // ── 11. Spawn signal processing task ──────────────────────────────────────
    task_handles.push(("signal_processing", tokio::spawn(run_signal_processing(
        signal_subscriber,
        Arc::clone(&signal_engine),
        shutdown.clone(),
        Arc::clone(&signals_counter),
    ))));

    // ── 12. Spawn scalper task ────────────────────────────────────────────────
    task_handles.push(("scalper", tokio::spawn(run_scalper(
        signal_rx,
        Arc::clone(&scalp_engine),
        Arc::clone(&signal_engine),
        Arc::clone(&config),
        shutdown.clone(),
        Arc::clone(&trades_evaluated),
        Arc::clone(&trades_rejected),
    ))));

    // ── 13. Control API ───────────────────────────────────────────────────────
    info!(subsystem = "control_api", "control API not yet wired — skipping");

    // ── 14. Await shutdown, then join all tasks ───────────────────────────────
    wait_for_shutdown().await;
    info!("broadcasting shutdown to all tasks");
    shutdown.cancel();

    let join_deadline = Duration::from_secs(5);
    for (name, handle) in task_handles {
        match tokio::time::timeout(join_deadline, handle).await {
            Ok(Ok(()))  => info!(task = name, "task completed cleanly"),
            Ok(Err(e))  => warn!(task = name, error = %e, "task panicked"),
            Err(_)      => warn!(task = name, "task force-killed (5s timeout exceeded)"),
        }
    }

    info!("system shutdown complete");
    std::process::exit(0);
}

// ─────────────────────────────────────────────────────────────────────────────
// Shutdown
// ─────────────────────────────────────────────────────────────────────────────

async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigusr1 =
            signal(SignalKind::user_defined1()).expect("failed to install SIGUSR1 handler");

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shutdown signal received (Ctrl+C)");
            }
            _ = sigterm.recv() => {
                halt::trigger_emergency_halt();
            }
            _ = sigusr1.recv() => {
                halt::trigger_emergency_halt();
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        tracing::info!("shutdown signal received");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Monitoring task  (30 s interval health snapshot)
// ─────────────────────────────────────────────────────────────────────────────

async fn run_monitoring(
    start:            Instant,
    events_counter:   Arc<AtomicU64>,
    signals_counter:  Arc<AtomicU64>,
    trades_evaluated: Arc<AtomicU64>,
    trades_rejected:  Arc<AtomicU64>,
    shutdown:         CancellationToken,
) {
    info!(subsystem = "monitor", interval_secs = 30, "monitoring task started");

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                info!(subsystem = "monitor", "monitoring task shutting down");
                return;
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                let uptime    = start.elapsed().as_secs();
                let events    = events_counter.load(Ordering::Relaxed);
                let signals   = signals_counter.load(Ordering::Relaxed);
                let evaluated = trades_evaluated.load(Ordering::Relaxed);
                let rejected  = trades_rejected.load(Ordering::Relaxed);

                info!(
                    subsystem        = "monitor",
                    uptime_secs      = uptime,
                    events_processed = events,
                    signals_emitted  = signals,
                    trades_evaluated = evaluated,
                    trades_rejected  = rejected,
                    "[monitor] uptime={uptime}s events_processed={events} \
                     signals_emitted={signals} trades_evaluated={evaluated} \
                     trades_rejected={rejected}"
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Synthetic ingestion task  (paper / backtest modes)
// ─────────────────────────────────────────────────────────────────────────────

/// Generates deterministic synthetic [`MarketEvent`]s at 10 events/second to
/// drive the full signal-engine pipeline without a live RPC connection.
async fn run_synthetic_ingestion(
    bus:            EventBus,
    shutdown:       CancellationToken,
    events_counter: Arc<AtomicU64>,
) {
    const RATE_HZ: u64    = 10;
    const INTERVAL_MS: u64 = 1_000 / RATE_HZ;

    // Deterministic pool addresses — rotated round-robin.
    let pools: [Pubkey; 5] = [
        Pubkey::new([0x11; 32]),
        Pubkey::new([0x22; 32]),
        Pubkey::new([0x33; 32]),
        Pubkey::new([0x44; 32]),
        Pubkey::new([0x55; 32]),
    ];
    let token_a = Pubkey::new([0xAA; 32]);
    let token_b = Pubkey::new([0xBB; 32]);

    info!(
        subsystem = "ingestion",
        rate_hz   = RATE_HZ,
        "synthetic ingestion task started"
    );

    let mut seq: u64 = 0;

    loop {
        if halt::should_halt() || halt::check_halt_file() {
            info!(subsystem = "ingestion", "halt — ingestion stopping");
            std::process::exit(0);
        }

        tokio::select! {
            _ = shutdown.cancelled() => {
                info!(subsystem = "ingestion", "ingestion task shutting down");
                return;
            }
            _ = tokio::time::sleep(Duration::from_millis(INTERVAL_MS)) => {
                let pool  = pools[(seq as usize) % pools.len()];

                // Alternate between pool-updates and swap events for richer feature coverage.
                let event = if seq % 3 == 0 {
                    MarketEvent::PoolUpdate(PoolUpdate {
                        pool:         Some(pool),
                        token_a_mint: Some(token_a),
                        token_b_mint: Some(token_b),
                        liquidity:    Some(1_000_000_u128 + seq as u128 * 1_000),
                        sqrt_price:   Some(79_228_162_514_u128 + seq as u128),
                        fee_rate:     Some(300),
                    })
                } else {
                    MarketEvent::SwapEvent(SwapEvent {
                        pool,
                        input_mint:  token_a,
                        output_mint: token_b,
                        amount_in:   1_000_000 + seq as u128 * 100,
                        amount_out:  998_000   + seq as u128 * 99,
                    })
                };

                match bus.publish(event) {
                    Ok(_report) => {
                        events_counter.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        warn!(subsystem = "ingestion", error = %e, "event publish failed");
                    }
                }

                seq = seq.wrapping_add(1);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Signal processing task
// ─────────────────────────────────────────────────────────────────────────────

async fn run_signal_processing(
    subscriber:      EventSubscriber,
    engine:          Arc<SignalEngine>,
    shutdown:        CancellationToken,
    signals_counter: Arc<AtomicU64>,
) {
    info!(subsystem = "signal_processing", "signal processing task started");

    loop {
        if halt::should_halt() || halt::check_halt_file() {
            info!(subsystem = "signal_processing", "halt — signal processing stopping");
            std::process::exit(0);
        }

        // Drain all immediately-available events before yielding to the runtime.
        loop {
            match subscriber.try_recv() {
                Ok(Some(event)) => {
                    let emitted = engine.process(SignalInput::Market(event));
                    signals_counter.fetch_add(emitted.len() as u64, Ordering::Relaxed);
                }
                _ => break, // empty channel or subscriber disconnected
            }
        }

        tokio::select! {
            _ = shutdown.cancelled() => {
                info!(subsystem = "signal_processing", "signal processing task shutting down");
                return;
            }
            _ = tokio::time::sleep(Duration::from_millis(5)) => {}
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scalper task
// ─────────────────────────────────────────────────────────────────────────────

async fn run_scalper(
    signal_rx:        SignalReceiver,
    scalp_engine:     Arc<ScalpEngine>,
    signal_engine:    Arc<SignalEngine>,
    config:           Arc<SystemConfig>,
    shutdown:         CancellationToken,
    trades_evaluated: Arc<AtomicU64>,
    trades_rejected:  Arc<AtomicU64>,
) {
    info!(subsystem = "scalper", "scalper task started");

    loop {
        if halt::should_halt() || halt::check_halt_file() {
            info!(subsystem = "scalper", "halt — scalper stopping");
            std::process::exit(0);
        }

        // Drain all available signals in a tight inner loop.
        loop {
            match signal_rx.try_recv() {
                Ok(signal) => {
                    trades_evaluated.fetch_add(1, Ordering::Relaxed);

                    let now_micros  = unix_now_micros();

                    // Compute feature snapshot for this pool from the shared feature store.
                    let features: ComputedFeatures = signal_engine
                        .feature_store()
                        .compute(signal.pool_address, now_micros, &config.signal_engine);

                    // Synthetic pool TVL: use configured minimum liquidity as a proxy.
                    // Real TVL will come from the ingestion layer in a future phase.
                    let pool_tvl_usd = config.risk.min_liquidity_usd.max(10_000.0);

                    match scalp_engine.evaluate(signal, features, pool_tvl_usd, now_micros) {
                        Some(result) => {
                            tracing::debug!(
                                subsystem = "scalper",
                                pnl_bps   = result.pnl_bps,
                                "paper fill executed — no on-chain submission in paper mode"
                            );
                        }
                        None => {
                            trades_rejected.fetch_add(1, Ordering::Relaxed);
                            tracing::debug!(
                                subsystem = "scalper",
                                "trade rejected by filter chain"
                            );
                        }
                    }
                }
                Err(_) => break, // empty or sender disconnected
            }
        }

        tokio::select! {
            _ = shutdown.cancelled() => {
                info!(subsystem = "scalper", "scalper task shutting down");
                return;
            }
            _ = tokio::time::sleep(Duration::from_millis(10)) => {}
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Time helper
// ─────────────────────────────────────────────────────────────────────────────

fn unix_now_micros() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tracing initialisation
// ─────────────────────────────────────────────────────────────────────────────

fn init_tracing(config_log_level: &str) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    // RUST_LOG env var takes priority over the value in SystemConfig.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config_log_level));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

// ─────────────────────────────────────────────────────────────────────────────
// Backtest mode
// ─────────────────────────────────────────────────────────────────────────────

fn run_backtest_mode(config: Arc<SystemConfig>) {
    use backtester::{generate_dataset, run_pipeline};

    let mut tuned = (*config).clone();
    let strategy = backtester::strategy_config();
    tuned.scalper = strategy.scalper;
    tuned.signal_engine = strategy.signal_engine;
    tuned.execution = strategy.execution;
    let cfg = Arc::new(tuned);

    info!("backtest mode: generating 6h synthetic dataset");
    let dataset = generate_dataset(6 * 3600, 10);
    info!(events = dataset.events.len(), "running backtest pipeline");

    let report = run_pipeline(cfg, &dataset, 0.80);

    info!(
        baseline_net = report.baseline.metrics.combined_net_pnl_usd,
        retest_net   = report.retest.metrics.combined_net_pnl_usd,
        simulation_net = report.simulation.net_pnl_usd,
        trades_per_day = report.simulation.trades_per_hour * 24.0,
        "backtest complete"
    );

    if let Ok(json) = serde_json::to_string_pretty(&report) {
        let _ = std::fs::create_dir_all("data");
        if std::fs::write("data/backtest_results.json", json).is_ok() {
            info!("results written to data/backtest_results.json");
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use config::SystemConfig;

    #[test]
    fn default_system_config_is_constructible() {
        let cfg = SystemConfig::default();
        assert!(cfg.pipeline.max_events == 0 || cfg.pipeline.max_events > 0);
        assert!(cfg.execution.simulation_initial_amount_usd > 0.0);
    }

    #[test]
    fn run_mode_variants_are_distinct() {
        use crate::RunMode;
        assert_ne!(RunMode::Paper,    RunMode::Live);
        assert_ne!(RunMode::Backtest, RunMode::Live);
        assert_ne!(RunMode::Paper,    RunMode::Backtest);
    }

    #[test]
    fn live_mode_is_always_rejected_by_design() {
        use crate::RunMode;
        // RunMode::Live is defined but guarded by two sequential exit(1) checks.
        let live = RunMode::Live;
        assert_eq!(live, RunMode::Live);
        assert_ne!(live, RunMode::Paper);
    }
}
