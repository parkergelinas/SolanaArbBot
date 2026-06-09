//! Execution engine — async tokio service, paper mode by default.

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use execution_engine::{
    audit::AuditLog, config::EngineConfig, jupiter::JupiterExecutor, orders::OrderStore,
    router::ExecutionRouter, subscriber,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // ── Startup environment validation ────────────────────────────────────────
    // Fail fast with a clear error if required secrets are missing.
    {
        const REQUIRED_ENV: &[(&str, &str)] = &[
            ("SOLANA_ARB_WALLET_KEY", "Solana wallet signing key (base58 64-byte keypair)"),
            ("GEYSER_GRPC_ENDPOINT", "Yellowstone gRPC endpoint URL"),
        ];
        let mut missing = false;
        for (var, description) in REQUIRED_ENV {
            if std::env::var(var).is_err() {
                tracing::error!(
                    env_var = var,
                    description,
                    "required environment variable not set — cannot start"
                );
                missing = true;
            }
        }
        if missing {
            tracing::error!(
                "one or more required env vars are missing; \
                 set them and restart the execution engine"
            );
            std::process::exit(1);
        }
    }

    let config = EngineConfig::from_env();
    info!(
        paper_mode = config.paper_mode,
        execution_live = config.execution_live,
        max_position_usd = config.max_position_usd,
        "execution-engine starting"
    );

    if config.execution_live {
        tracing::warn!("EXECUTION_LIVE=true — real swap transactions may be built");
    }

    let orders = Arc::new(OrderStore::new());
    let audit = Arc::new(AuditLog::new(config.audit_path.clone()));
    let jupiter = Arc::new(JupiterExecutor::new(config.clone()));
    jupiter.prewarm().await?;

    let (signal_tx, signal_rx) = mpsc::unbounded_channel();
    let (lifecycle_tx, mut lifecycle_rx) = mpsc::unbounded_channel();

    let router = Arc::new(ExecutionRouter::new(
        config.clone(),
        orders.clone(),
        audit,
        jupiter,
        Some(lifecycle_tx),
    ));
    router.spawn(signal_rx);

    tokio::spawn(async move {
        while lifecycle_rx.recv().await.is_some() {}
    });

    let demo_interval: u64 = std::env::var("DEMO_SIGNAL_INTERVAL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5_000);

    let use_hub = std::env::var("SIGNAL_HUB_URL")
        .or_else(|_| std::env::var("CONTROL_API_URL"))
        .is_ok()
        || std::env::var("EXECUTION_SIGNAL_HUB")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false);

    if use_hub {
        subscriber::spawn_signal_hub_subscriber(signal_tx.clone());
    } else {
        subscriber::spawn_intelligence_stub(signal_tx.clone());
    }

    if std::env::var("ALPHA_ENGINE_ENABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
    {
        let (alpha_tx, alpha_rx) = mpsc::unbounded_channel();
        subscriber::spawn_alpha_channel_bridge(alpha_rx, signal_tx.clone());
        info!("alpha-engine channel bridge enabled — wire alpha_tx from in-process spawn");
        let _ = alpha_tx;
    } else if let Ok(arb_url) = std::env::var("ARB_WS_URL") {
        subscriber::spawn_arb_ws_subscriber(arb_url, signal_tx.clone());
    } else {
        subscriber::spawn_demo_signal_feed(signal_tx, demo_interval);
    }

    info!("execution-engine running — awaiting signals");
    tokio::signal::ctrl_c().await?;
    info!("shutting down");
    Ok(())
}
