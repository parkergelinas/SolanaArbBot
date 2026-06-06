//! Optional DexScreener / Birdeye / Jupiter pollers for enrichment.

use std::sync::Arc;

use config::ConfigHandle;
use events::EventBus;
use pricing::MintWatchlist;
use signal_bus::adapter::SOL_MINT;
use tracing::{info, warn};

const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

fn pollers_enabled() -> bool {
    std::env::var("STREAM_EXTERNAL_POLLERS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Start `signals::spawn_live_data_pollers` when `STREAM_EXTERNAL_POLLERS=1`.
pub fn spawn_optional_external_pollers() {
    if !pollers_enabled() {
        return;
    }

    tokio::spawn(async {
        let handle = match ConfigHandle::load() {
            Ok(h) => h,
            Err(e) => {
                warn!(error = %e, "external pollers: config load failed");
                return;
            }
        };

        let data_sources = Arc::new(handle.data_sources.clone());
        let bus = EventBus::new();
        let mints: MintWatchlist = Arc::new(vec![
            SOL_MINT.to_owned(),
            USDC_MINT.to_owned(),
        ]);

        let whale_cfg = Arc::new(handle.whale_tracker.clone());
        let _handles = signals::spawn_live_data_pollers(bus, data_sources, mints, Some(whale_cfg))
            .await;
        info!("stream-api external data pollers started (STREAM_EXTERNAL_POLLERS=1)");
    });
}
