//! Always-on Solscan researcher + pump.fun scanner pollers for stream-api REST.

use std::sync::Arc;

use ratelimit::RateLimiter;
use signals::{spawn_pump_scanner, spawn_solscan_researcher, ScannerStore};
use tracing::info;

/// Start scanner pollers (DexScreener-backed; no API key required for basic mode).
pub fn spawn_scanner_pollers(store: ScannerStore) {
    let data_sources = Arc::new(
        config::ConfigHandle::load()
            .map(|h| h.data_sources.clone())
            .unwrap_or_default(),
    );
    let limiter = Arc::new(RateLimiter::free_tier_defaults());

    spawn_solscan_researcher(store.clone(), Arc::clone(&data_sources), Arc::clone(&limiter));
    spawn_pump_scanner(store, Arc::clone(&data_sources), Arc::clone(&limiter));
    info!("stream-api scanner pollers started (solscan researcher + pump.fun)");
}
