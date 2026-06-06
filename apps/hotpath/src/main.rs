//! Ultra low-latency single-process hot-path binary.
//!
//! ```bash
//! cargo run -p hotpath
//! ```
//!
//! ## Configuration
//!
//! All settings come from `config.toml` (or `SOLANA_ARB_CONFIG` env var) plus
//! environment variable overrides:
//!
//! | Env var                             | Effect                                 |
//! |-------------------------------------|----------------------------------------|
//! | `SOLANA_ARB_DATA_SOURCES__HELIUS_API_KEY` | Enable real market data (required) |
//! | `SOLANA_ARB_WALLET_KEY`             | Base58 keypair for live trading        |
//! | `SOLANA_ARB_CONFIRM_LIVE_TRADING=1` | Required to disable paper mode         |
//! | `SOLANA_ARB_HOTPATH__PAPER_MODE=false` | Disable paper mode in config        |
//! | `RUST_LOG=info`                     | Log level                              |
//!
//! ## Architecture
//!
//! ```text
//! [Helius WS accountSubscribe]
//!        │  real SPL vault amounts → MarketTick
//!        ▼
//! [Ingestion Thread] ──SPSC──► [Hot Loop Thread] ──SPSC──► [Cold I/O Thread]
//!                                                               │
//!                                                    Jupiter quote → swap tx
//!                                                    Jito tip tx + swap tx
//!                                                               │
//!                                                         Jito block-engine
//! ```

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use config::ConfigHandle;
use engine::{
    hotpath_runtime::{spawn_monitor, spawn_synthetic_ingestion, HotPathRuntime},
};
use ingestion::spawn_helius_hotpath_feed;
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = ConfigHandle::load().expect("failed to load SystemConfig");

    info!(
        paper = cfg.hotpath.paper_mode,
        budget_us = cfg.hotpath.decision_budget_us,
        prefer_jito = cfg.hotpath.prefer_jito,
        helius_key_set = !cfg.data_sources.helius_api_key.is_empty(),
        "starting hot-path engine"
    );

    let runtime = Arc::new(HotPathRuntime::start_with_config(&cfg.hotpath, &*cfg));
    let tick_tx = runtime.tick_sender();

    // ── Market data feed ─────────────────────────────────────────────────────
    // Use real Helius accountSubscribe feed when an API key is configured.
    // Falls back to synthetic data so the engine always runs in dev/paper mode.
    let helius_key_set = !cfg.data_sources.helius_api_key.is_empty();

    if helius_key_set {
        info!("starting real Helius accountSubscribe market feed");
        // The Helius feed is async — start a tokio runtime for it.
        let data_sources = Arc::new(cfg.data_sources.clone());
        let tick_tx_clone = tick_tx.clone();
        thread::Builder::new()
            .name("helius-async".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("helius tokio runtime");
                rt.block_on(async {
                    spawn_helius_hotpath_feed(tick_tx_clone, data_sources);
                    // Keep the runtime alive indefinitely.
                    loop {
                        tokio::time::sleep(Duration::from_secs(3600)).await;
                    }
                });
            })
            .expect("spawn helius-async");
    } else {
        info!("no Helius API key — using synthetic ingestion (paper mode only)");
        spawn_synthetic_ingestion(tick_tx, 5);
    }

    spawn_monitor(Arc::clone(&runtime));

    info!("hot-path running — press Ctrl+C to stop");
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}
