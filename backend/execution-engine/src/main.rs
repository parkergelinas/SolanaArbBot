//! Execution engine service — consumes trade signals, routes to Jupiter (paper by default).

use std::sync::Arc;

use crossbeam_channel::unbounded;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use execution_engine::{
    audit::AuditLog, config::EngineConfig, orders::OrderStore, router::ExecutionRouter,
    subscriber,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = EngineConfig::from_env();
    info!(
        paper_mode = config.paper_mode,
        min_confidence = config.min_confidence,
        max_size_usd = config.max_size_usd,
        "execution-engine starting"
    );

    if !config.paper_mode {
        tracing::warn!(
            "PAPER_MODE=false — live trading path enabled; ensure EXECUTION_KEYPAIR_PATH is set"
        );
    }

    let orders = Arc::new(OrderStore::new());
    let audit = Arc::new(AuditLog::new(config.audit_path.clone()));
    let (signal_tx, signal_rx) = unbounded();
    let (lifecycle_tx, lifecycle_rx) = unbounded();

    let router = Arc::new(ExecutionRouter::new(
        config.clone(),
        orders.clone(),
        audit,
        Some(lifecycle_tx),
    ));
    router.spawn_worker(signal_rx);

    std::thread::spawn(move || {
        while let Ok(ev) = lifecycle_rx.recv() {
            tracing::debug!(?ev, "lifecycle event");
        }
    });

    let demo_interval: u64 = std::env::var("DEMO_SIGNAL_INTERVAL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5_000);

    subscriber::spawn_intelligence_stub(signal_tx.clone());
    subscriber::spawn_demo_signal_feed(signal_tx, demo_interval);

    info!("execution-engine running — awaiting signals");
    tokio::signal::ctrl_c().await?;
    info!("shutting down");
    Ok(())
}
