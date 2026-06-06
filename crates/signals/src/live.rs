//! Spawn all free-tier external data pollers.

use std::sync::Arc;

use config::{DataSourcesConfig, WhaleTrackerConfig};
use events::EventBus;
use pricing::{spawn_jupiter_poller, MintWatchlist};
use ratelimit::RateLimiter;

use crate::{
    birdeye::{load_jupiter_verified, spawn_birdeye_top_movers_poller, spawn_birdeye_whale_poller},
    dexscreener::{spawn_dexscreener_poller, spawn_volume_spike_poller},
    external_store::ExternalSignalStore,
    whale_discovery::{build_tracked_wallet_set, spawn_whale_discovery},
    whale_watcher::{spawn_whale_watcher, WhaleSignalStore, WHALE_WALLETS},
};

/// Handles returned by live data poller startup.
#[derive(Clone)]
pub struct LiveDataHandles {
    pub external: ExternalSignalStore,
    pub whale: WhaleSignalStore,
}

/// Start DexScreener, Birdeye, Jupiter, and optional whale tracker pollers.
pub async fn spawn_live_data_pollers(
    bus: EventBus,
    data_sources: Arc<DataSourcesConfig>,
    jupiter_mints: MintWatchlist,
    whale_cfg: Option<Arc<WhaleTrackerConfig>>,
) -> LiveDataHandles {
    let limiter = Arc::new(RateLimiter::free_tier_defaults());
    let store = ExternalSignalStore::new();
    let whale_store = WhaleSignalStore::new();

    load_jupiter_verified(&store).await;

    spawn_dexscreener_poller(store.clone(), Arc::clone(&data_sources), Arc::clone(&limiter));
    spawn_volume_spike_poller(
        store.clone(),
        Arc::clone(&data_sources),
        Arc::clone(&limiter),
        jupiter_mints.clone(),
    );
    spawn_birdeye_top_movers_poller(store.clone(), Arc::clone(&data_sources), Arc::clone(&limiter));

    let wallets: Arc<Vec<String>> =
        Arc::new(WHALE_WALLETS.iter().map(|s| (*s).to_owned()).collect());
    spawn_birdeye_whale_poller(Arc::clone(&data_sources), Arc::clone(&limiter), wallets);

    if let Some(cfg) = whale_cfg.filter(|c| c.enabled) {
        let tracked = build_tracked_wallet_set(&cfg);
        spawn_whale_discovery(Arc::clone(&tracked), Arc::clone(&cfg));
        spawn_whale_watcher(
            whale_store.clone(),
            tracked,
            Arc::clone(&data_sources),
            cfg,
            Arc::clone(&limiter),
        );
    }

    spawn_jupiter_poller(bus, data_sources, jupiter_mints, limiter);

    LiveDataHandles {
        external: store,
        whale: whale_store,
    }
}
