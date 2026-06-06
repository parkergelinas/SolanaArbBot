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
pub mod processor;
pub mod processors;
pub mod types;
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
pub use external_store::{ExternalSignalStore, NewTokenSignal, WhaleActivitySignal};
pub use live::{spawn_live_data_pollers, LiveDataHandles};
pub use whale_discovery::{build_tracked_wallet_set, load_discovered_wallets, spawn_whale_discovery};
pub use whale_watcher::{
    spawn_whale_watcher, short_wallet, DexSource, WhaleSignalStore, WhaleSwapSignal, WHALE_WALLETS,
};
pub use feature_store::{ComputedFeatures, FeatureStore};
pub use processor::SignalProcessor;
pub use processors::{MomentumProcessor, SmartMoneyProcessor, WhaleFlowProcessor};
pub use types::{
    Direction, FeatureVector, SignalEvent, SignalInput, SignalType, WhaleEvent,
};
