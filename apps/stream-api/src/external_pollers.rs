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

        let whale_cfg = if handle.whale_tracker.enabled {
            Some(Arc::new(handle.whale_tracker.clone()))
        } else {
            None
        };
        let copy_cfg = if handle.copy_trading.enabled {
            Some(Arc::new(handle.copy_trading.clone()))
        } else {
            None
        };
        let liquidation_cfg = if handle.liquidation.enabled {
            Some(Arc::new(handle.liquidation.clone()))
        } else {
            None
        };
        let momentum_cfg = if handle.momentum.enabled {
            Some(Arc::new(handle.momentum.clone()))
        } else {
            None
        };
        let features = Arc::new(handle.features.clone());
        let _handles = signals::spawn_live_data_pollers(
            bus,
            data_sources,
            mints,
            whale_cfg,
            copy_cfg,
            liquidation_cfg,
            momentum_cfg,
            Some(features),
            None,
        )
        .await;
        info!("stream-api external data pollers started (STREAM_EXTERNAL_POLLERS=1)");
    });
}
