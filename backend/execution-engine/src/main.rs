//! Execution engine — async tokio service, paper mode by default.
//!
//! # Live trading
//! Set `EXECUTION_LIVE=true` and rebuild with `--features live-signing`.
//! Requires OpenSSL (Linux: install via package manager; Windows: `choco install openssl`).

use std::sync::Arc;

use axum::{routing::post, Json, Router};
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use execution_engine::{
    audit::AuditLog, config::EngineConfig, jupiter::JupiterExecutor, orders::OrderStore,
    router::ExecutionRouter, signals::TradeSignal, subscriber,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = EngineConfig::from_env();

    // ── Live-mode env validation ───────────────────────────────────────────────
    // Only enforce hard requirements when actually submitting transactions.
    // Paper / demo mode has full fallback paths and needs neither.
    if config.execution_live {
        if std::env::var("SOLANA_ARB_WALLET_KEY").is_err() {
            tracing::error!(
                env_var = "SOLANA_ARB_WALLET_KEY",
                "required for live trading — set wallet signing key and restart"
            );
            std::process::exit(1);
        }
    }
    info!(
        paper_mode = config.paper_mode,
        execution_live = config.execution_live,
        max_position_usd = config.max_position_usd,
        "execution-engine starting"
    );

    if config.execution_live {
        tracing::warn!("EXECUTION_LIVE=true — real swap transactions will be built and submitted");
    }

    // ── Wallet loading ────────────────────────────────────────────────────────
    // Wallet is only loaded when the `live-signing` feature is compiled in AND
    // EXECUTION_LIVE=true.  In all other cases paper mode is active.
    #[cfg(feature = "live-signing")]
    let wallet = if config.execution_live {
        let mut sys_cfg = config::SystemConfig::default();
        sys_cfg.features.dry_run = false;
        match wallet::WalletKeypair::load_from_env("SOLANA_ARB_WALLET_KEY", &sys_cfg) {
            Ok(kp) => {
                info!("live wallet keypair loaded");
                Some(Arc::new(kp))
            }
            Err(e) => {
                tracing::error!(error = %e, "wallet load failed in live mode — aborting");
                std::process::exit(1);
            }
        }
    } else {
        info!("paper mode — wallet keypair not loaded");
        None
    };

    #[cfg(not(feature = "live-signing"))]
    let wallet = ();

    let orders = Arc::new(OrderStore::new());
    let audit = Arc::new(AuditLog::new(config.audit_path.clone()));
    let jupiter = Arc::new(JupiterExecutor::new(config.clone(), wallet));
    jupiter.prewarm().await?;

    let (signal_tx, signal_rx) = mpsc::unbounded_channel::<TradeSignal>();
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

    // ── HTTP signal ingest ────────────────────────────────────────────────────
    // Arb-engine posts TradeSignalOut JSON here when EXECUTION_ENGINE_URL is set.
    // TradeSignalOut and TradeSignal share the same JSON schema, so no
    // field mapping is needed.
    let http_port: u16 = std::env::var("EXECUTION_HTTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8090);

    {
        let signal_tx_http = signal_tx.clone();
        tokio::spawn(async move {
            let app = Router::new().route(
                "/api/signals",
                post(move |Json(payload): Json<TradeSignal>| {
                    let tx = signal_tx_http.clone();
                    async move {
                        if tx.send(payload).is_err() {
                            return axum::http::StatusCode::SERVICE_UNAVAILABLE;
                        }
                        axum::http::StatusCode::ACCEPTED
                    }
                }),
            );

            match tokio::net::TcpListener::bind(format!("0.0.0.0:{http_port}")).await {
                Ok(listener) => {
                    info!(port = http_port, "execution-engine HTTP signal ingest listening");
                    axum::serve(listener, app).await.ok();
                }
                Err(e) => {
                    tracing::warn!(port = http_port, error = %e, "HTTP signal ingest failed to bind");
                }
            }
        });
    }

    // ── Signal source selection ───────────────────────────────────────────────
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

    if let Ok(arb_url) = std::env::var("ARB_WS_URL") {
        // Primary arb→execution WS path: arb-engine broadcasts ArbSignal, execution-engine
        // subscribes and converts to TradeSignal.  Set ARB_WS_URL=ws://127.0.0.1:8091/arb
        subscriber::spawn_arb_ws_subscriber(arb_url, signal_tx.clone());
    } else {
        subscriber::spawn_demo_signal_feed(signal_tx, demo_interval);
    }

    info!(
        "execution-engine running — HTTP ingest on :{http_port}, awaiting signals"
    );
    tokio::signal::ctrl_c().await?;
    info!("shutting down");
    Ok(())
}
