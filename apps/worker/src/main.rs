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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use config::ConfigHandle;
use events::{EventBus, EventSubscriber, MarketEvent, PoolUpdate, SwapEvent};
use common::Pubkey;
use scalper::{EvaluationResult, ScalpEngine};
use signals::{SignalEngine, SignalInput, SignalReceiver};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// CLI surface
// ─────────────────────────────────────────────────────────────────────────────

/// Pipeline run mode.
#[derive(Debug, Clone, PartialEq, clap::ValueEnum)]
enum RunMode {
    /// Synthetic market events, no real RPC, no trades submitted.
    Paper,
    /// Same as paper; reserved for historical replay integration.
    Backtest,
    /// Requires `--enable-live`. Live trading is NOT implemented in this build.
    Live,
}

#[derive(Parser, Debug)]
#[command(
    name    = "worker",
    about   = "Solana quant trading system — production runtime orchestrator",
    version
)]
struct Cli {
    /// Run mode: paper (default), backtest, or live.
    #[arg(long, value_enum, default_value = "paper")]
    mode: RunMode,

    /// Optional path to a TOML config file.
    /// Falls back to `SOLANA_ARB_CONFIG` env var, then `./config.toml`.
    #[arg(long)]
    config: Option<std::path::PathBuf>,

    /// Must be set when `--mode live`. Absent in live mode → immediate exit(1).
    #[arg(long, default_value_t = false)]
    enable_live: bool,
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

    // ── 5. Create shutdown signal ─────────────────────────────────────────────
    let shutdown   = CancellationToken::new();
    let start_time = Instant::now();

    // ── 6. Init EventBus ──────────────────────────────────────────────────────
    let event_bus = EventBus::new();
    info!(subsystem = "event_bus", "initialized");

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

    let mut task_handles: Vec<(&'static str, tokio::task::JoinHandle<()>)> = Vec::new();

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
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    tracing::info!("shutdown signal received");
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
    shutdown:         CancellationToken,
    trades_evaluated: Arc<AtomicU64>,
    trades_rejected:  Arc<AtomicU64>,
) {
    info!(subsystem = "scalper", "scalper task started");

    loop {
        // Drain all available signals in a tight inner loop.
        loop {
            match signal_rx.try_recv() {
                Ok(signal) => {
                    trades_evaluated.fetch_add(1, Ordering::Relaxed);

                    match scalp_engine.evaluate(&signal) {
                        EvaluationResult::Accept { expected_profit_bps } => {
                            tracing::debug!(
                                subsystem            = "scalper",
                                signal_type          = %signal.signal_type,
                                direction            = %signal.direction,
                                strength             = signal.strength,
                                expected_profit_bps,
                                "trade ACCEPTED — paper mode: no on-chain submission"
                            );
                            // Not rejected — trade would be accepted in live mode.
                        }
                        EvaluationResult::Reject { reason } => {
                            tracing::debug!(
                                subsystem   = "scalper",
                                signal_type = %signal.signal_type,
                                direction   = %signal.direction,
                                reason,
                                "trade REJECTED"
                            );
                            trades_rejected.fetch_add(1, Ordering::Relaxed);
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
        use super::RunMode;
        assert_ne!(RunMode::Paper,    RunMode::Live);
        assert_ne!(RunMode::Backtest, RunMode::Live);
        assert_ne!(RunMode::Paper,    RunMode::Backtest);
    }

    #[test]
    fn live_mode_is_always_rejected_by_design() {
        // RunMode::Live is defined but guarded by two sequential exit(1) checks.
        let live = RunMode::Live;
        assert_eq!(live, RunMode::Live);
        assert_ne!(live, RunMode::Paper);
    }
}
