//! Ultra low-latency single-process hot-path binary.
//!
//! ```bash
//! cargo run -p hotpath
//! ```
//!
//! Architecture:
//! ```text
//! ingestion thread → SPSC channel → hot loop thread → SPSC queue → cold I/O thread
//! ```
//!
//! No HTTP, no WebSocket, no DB — pure in-process micro-arbitrage pipeline.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use config::ConfigHandle;
use engine::{spawn_monitor, spawn_synthetic_ingestion, HotPathRuntime};
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
        "starting hot-path engine"
    );

    let runtime = Arc::new(HotPathRuntime::start(&cfg.hotpath));
    spawn_synthetic_ingestion(runtime.tick_sender(), 5);
    spawn_monitor(Arc::clone(&runtime));

    info!("hot-path running — press Ctrl+C to stop");

    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}
