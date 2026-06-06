//! Modular signal engine for the Solana market-analysis pipeline.
//!
//! # Event flow
//!
//! ```text
//! SignalInput (MarketEvent | WhaleEvent)
//!   │
//!   ▼
//! FeatureStore::update_*      ← rolling windows updated
//!   │
//!   ▼
//! [WhaleFlowProcessor
//!  SmartMoneyProcessor          ← processors run in parallel over ComputedFeatures
//!  MomentumProcessor]
//!   │
//!   ▼
//! SignalAggregator::filter    ← dedup / cooldown / strength+confidence gates
//!   │
//!   ▼
//! SignalSender (crossbeam)    ← survivors published to the bus
//! ```
//!
//! # Quick start
//!
//! ```rust,no_run
//! use signals::{SignalEngine, SignalInput};
//! use config::SignalEngineConfig;
//! use common::MarketEvent;
//!
//! let (engine, receiver) = SignalEngine::new(SignalEngineConfig::default());
//!
//! // Feed market events:
//! engine.process(SignalInput::Market(/* MarketEvent */
//! # MarketEvent::SwapEvent(common::SwapEvent {
//! #     pool: common::Pubkey::new([0;32]),
//! #     input_mint: common::Pubkey::new([0;32]),
//! #     output_mint: common::Pubkey::new([0;32]),
//! #     amount_in: 100,
//! #     amount_out: 99,
//! # })
//! ));
//!
//! // Consume emitted signals:
//! while let Ok(signal) = receiver.try_recv() {
//!     println!("{}", signal.explanation);
//! }
//! ```

#![forbid(unsafe_code)]

pub mod aggregator;
pub mod birdeye;
pub mod bus;
pub mod dexscreener;
pub mod engine;
pub mod external_store;
pub mod feature_store;
pub mod live;
pub mod liquidation;
pub mod momentum;
pub mod quote_arb;
pub mod pump_scanner;
pub mod scanner_store;
pub mod solscan_researcher;
pub mod processor;
pub mod processors;
pub mod types;
pub mod copy_trader;
pub mod sniper;
pub mod wallet_scoring;
pub mod whale_discovery;
pub mod whale_watcher;

// ─────────────────────────────────────────────────────────────────────────────
// Flat re-exports — everything a caller needs at the crate root
// ─────────────────────────────────────────────────────────────────────────────

pub use aggregator::SignalAggregator;
pub use birdeye::{
    load_jupiter_verified, spawn_birdeye_top_movers_poller, spawn_birdeye_whale_poller,
};
pub use bus::{SignalReceiver, SignalSender, signal_channel};
pub use dexscreener::{rugcheck_passes, spawn_dexscreener_poller, spawn_volume_spike_poller};
pub use engine::SignalEngine;
pub use external_store::{ExternalSignalStore, NewTokenSignal, VolumeSpikeSignal, WhaleActivitySignal};
pub use momentum::{
    momentum_confidence, momentum_signal_to_bus_payload, price_velocity_pct_min, spawn_momentum_poller,
    volume_ratio, MomentumSignal, MomentumTraderState, MOMENTUM_SLIPPAGE_BPS,
};
pub use quote_arb::{
    fetch_reference_prices, scan_pair_divergence, spawn_quote_arb_poller, spawn_quote_arb_publisher,
    QuoteArbSignal,
};
pub use copy_trader::{
    copy_signal_channel, copy_signal_to_bus_payload, execute_copy, plan_copy, spawn_copy_trader,
    CopyExecutionPlan, CopySignal, CopySkipReason, CopyTraderState, RecentSalesTracker,
    COPY_SLIPPAGE_BPS,
};
pub use sniper::{
    compute_position_sol, evaluate_exit, execute_buy, fetch_token_price_usd, pool_creation_channel,
    rug_check, spawn_sniper_bus_publisher, spawn_sniper_monitor, spawn_sniper_strategy, ExitAction,
    RugCheckCache, RugRejectReason, RugVerdict, SniperCandidate, SniperPosition,
    SniperPublisher, JUPITER_SWAP_API, SNIPER_SLIPPAGE_BPS,
};
pub use liquidation::{
    classify_health, compute_health, execute_liquidation, plan_liquidation,
    spawn_liquidation_hunter, sort_positions_by_urgency, BorrowPosition, HealthTier,
    LendingProtocol, LiquidationPlan, LiquidationStore,
};
pub use live::{spawn_live_data_pollers, LiveDataHandles};
pub use pump_scanner::spawn_pump_scanner;
pub use scanner_store::{PumpMomentumHit, ScannerMeta, ScannerStore, ShitcoinWhaleHit};
pub use solscan_researcher::spawn_solscan_researcher;
pub use wallet_scoring::{
    fetch_wallet_score, qualifies_wallet, score_from_gmgn_row, spawn_wallet_scoring_poller,
    QualifiedWalletSet, WalletScore,
};
pub use whale_discovery::{build_tracked_wallet_set, load_discovered_wallets, spawn_whale_discovery};
pub use whale_watcher::{
    spawn_whale_watcher, short_wallet, CopyWatcherHooks, DexSource, WhaleSignalStore,
    WhaleSwapSignal, WHALE_WALLETS,
};
pub use feature_store::{ComputedFeatures, FeatureStore};
pub use processor::SignalProcessor;
pub use processors::{MomentumProcessor, SmartMoneyProcessor, WhaleFlowProcessor};
pub use types::{
    Direction, FeatureVector, SignalEvent, SignalInput, SignalType, WhaleEvent,
};
