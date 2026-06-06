//! Spawn all free-tier external data pollers.

use std::sync::Arc;

use config::{
    CopyTradingConfig, DataSourcesConfig, FeatureFlags, LiquidationConfig, MomentumConfig,
    QuoteArbConfig, WhaleTrackerConfig,
};
use crossbeam_channel::Sender;
use events::EventBus;
use pricing::{spawn_jupiter_poller, MintWatchlist, TokenQualityFilter};
use ratelimit::RateLimiter;

use crate::{
    birdeye::{load_jupiter_verified, spawn_birdeye_top_movers_poller, spawn_birdeye_whale_poller},
    copy_trader::{copy_signal_channel, spawn_copy_trader, CopySignal, CopyTraderState, RecentSalesTracker},
    dexscreener::{spawn_dexscreener_poller, spawn_volume_spike_poller},
    external_store::ExternalSignalStore,
    liquidation::{spawn_liquidation_hunter, LiquidationStore},
    momentum::spawn_momentum_poller,
    pump_scanner::spawn_pump_scanner,
    quote_arb::spawn_quote_arb_poller,
    scanner_store::ScannerStore,
    solscan_researcher::spawn_solscan_researcher,
    wallet_scoring::{spawn_wallet_scoring_poller, QualifiedWalletSet},
    whale_discovery::{build_tracked_wallet_set, spawn_whale_discovery},
    whale_watcher::{spawn_whale_watcher, CopyWatcherHooks, WhaleSignalStore, WHALE_WALLETS},
};

/// Handles returned by live data poller startup.
#[derive(Clone)]
pub struct LiveDataHandles {
    pub external: ExternalSignalStore,
    pub whale: WhaleSignalStore,
    pub scanners: ScannerStore,
}

/// Start DexScreener, Birdeye, Jupiter, whale tracker, scanners, and optional strategy pollers.
pub async fn spawn_live_data_pollers(
    bus: EventBus,
    data_sources: Arc<DataSourcesConfig>,
    jupiter_mints: MintWatchlist,
    whale_cfg: Option<Arc<WhaleTrackerConfig>>,
    copy_cfg: Option<Arc<CopyTradingConfig>>,
    liquidation_cfg: Option<Arc<LiquidationConfig>>,
    momentum_cfg: Option<Arc<MomentumConfig>>,
    quote_arb_cfg: Option<Arc<QuoteArbConfig>>,
    features: Option<Arc<FeatureFlags>>,
    strategy_copy_tx: Option<Sender<CopySignal>>,
) -> LiveDataHandles {
    let _ = bus;
    let limiter = Arc::new(RateLimiter::free_tier_defaults());
    let store = ExternalSignalStore::new();
    let whale_store = WhaleSignalStore::new();
    let scanner_store = ScannerStore::new();

    load_jupiter_verified(&store).await;

    spawn_dexscreener_poller(store.clone(), Arc::clone(&data_sources), Arc::clone(&limiter));
    spawn_volume_spike_poller(
        store.clone(),
        Arc::clone(&data_sources),
        Arc::clone(&limiter),
        jupiter_mints.clone(),
    );
    spawn_birdeye_top_movers_poller(store.clone(), Arc::clone(&data_sources), Arc::clone(&limiter));

    spawn_solscan_researcher(scanner_store.clone(), Arc::clone(&data_sources), Arc::clone(&limiter));
    spawn_pump_scanner(scanner_store.clone(), Arc::clone(&data_sources), Arc::clone(&limiter));

    let wallets: Arc<Vec<String>> =
        Arc::new(WHALE_WALLETS.iter().map(|s| (*s).to_owned()).collect());
    spawn_birdeye_whale_poller(Arc::clone(&data_sources), Arc::clone(&limiter), wallets);

    let features_arc = features.unwrap_or_else(|| Arc::new(FeatureFlags::default()));

    if let Some(cfg) = copy_cfg.as_ref().filter(|c| c.enabled) {
        let qualified = QualifiedWalletSet::new();
        spawn_wallet_scoring_poller(qualified.clone(), Arc::clone(cfg), 300);

        if strategy_copy_tx.is_none() {
            let (tx, rx) = copy_signal_channel(256);
            spawn_copy_trader(
                rx,
                Arc::clone(cfg),
                Arc::clone(&features_arc),
                Arc::clone(&data_sources),
                CopyTraderState::new(RecentSalesTracker::new()),
                Some(qualified.clone()),
            );
            let _ = tx;
        }
    }

    if let Some(cfg) = whale_cfg.filter(|c| c.enabled) {
        let tracked = build_tracked_wallet_set(&cfg);
        spawn_whale_discovery(Arc::clone(&tracked), Arc::clone(&cfg));

        let copy_hooks = copy_cfg.as_ref().filter(|c| c.enabled).and_then(|copy| {
            strategy_copy_tx.as_ref().map(|copy_tx| CopyWatcherHooks {
                copy_tx: copy_tx.clone(),
                copy_cfg: Arc::clone(copy),
                recent_sales: RecentSalesTracker::new(),
                qualified: Some(QualifiedWalletSet::new()),
            })
        });

        spawn_whale_watcher(
            whale_store.clone(),
            tracked,
            Arc::clone(&data_sources),
            cfg,
            Arc::clone(&limiter),
            copy_hooks,
        );
    }

    if let Some(cfg) = liquidation_cfg.filter(|c| c.enabled) {
        spawn_liquidation_hunter(
            cfg,
            Arc::clone(&features_arc),
            Arc::clone(&data_sources),
            Arc::clone(&limiter),
            LiquidationStore::new(),
        );
    }

    if let Some(cfg) = momentum_cfg.filter(|c| c.enabled) {
        spawn_momentum_poller(
            store.clone(),
            whale_store.clone(),
            cfg,
            Arc::clone(&features_arc),
            Arc::clone(&data_sources),
            Arc::clone(&limiter),
        );
    }

    if let Some(cfg) = quote_arb_cfg.filter(|c| c.enabled) {
        let quality = TokenQualityFilter::new();
        quality.refresh(&data_sources).await;
        let _ = spawn_quote_arb_poller(cfg, Arc::clone(&data_sources), quality);
    }

    spawn_jupiter_poller(
        bus,
        Arc::clone(&data_sources),
        jupiter_mints,
        Arc::clone(&limiter),
    );

    LiveDataHandles {
        external: store,
        whale: whale_store,
        scanners: scanner_store,
    }
}
