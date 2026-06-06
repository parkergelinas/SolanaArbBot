//! Spawn all free-tier external data pollers.

use std::sync::Arc;

use config::DataSourcesConfig;
use events::EventBus;
use pricing::{spawn_jupiter_poller, MintWatchlist};
use ratelimit::RateLimiter;

use crate::{
    birdeye::{load_jupiter_verified, spawn_birdeye_top_movers_poller, spawn_birdeye_whale_poller, DEFAULT_WHALE_WALLETS},
    dexscreener::{spawn_dexscreener_poller, spawn_volume_spike_poller},
    external_store::ExternalSignalStore,
};

/// Start DexScreener, Birdeye, and Jupiter pollers; returns shared external store for routing.
pub async fn spawn_live_data_pollers(
    bus: EventBus,
    data_sources: Arc<DataSourcesConfig>,
    jupiter_mints: MintWatchlist,
) -> ExternalSignalStore {
    let limiter = Arc::new(RateLimiter::free_tier_defaults());
    let store = ExternalSignalStore::new();

    load_jupiter_verified(&store).await;

    spawn_dexscreener_poller(store.clone(), Arc::clone(&data_sources), Arc::clone(&limiter));
    spawn_volume_spike_poller(
        store.clone(),
        Arc::clone(&data_sources),
        Arc::clone(&limiter),
        jupiter_mints.clone(),
    );
    spawn_birdeye_top_movers_poller(store.clone(), Arc::clone(&data_sources), Arc::clone(&limiter));

    let wallets: Arc<Vec<String>> = Arc::new(
        DEFAULT_WHALE_WALLETS.iter().map(|s| (*s).to_owned()).collect(),
    );
    spawn_birdeye_whale_poller(Arc::clone(&data_sources), Arc::clone(&limiter), wallets);

    spawn_jupiter_poller(bus, data_sources, jupiter_mints, limiter);

    store
}
