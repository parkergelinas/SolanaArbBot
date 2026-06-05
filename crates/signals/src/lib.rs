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
pub mod bus;
pub mod engine;
pub mod feature_store;
pub mod processor;
pub mod processors;
pub mod types;

// ─────────────────────────────────────────────────────────────────────────────
// Flat re-exports — everything a caller needs at the crate root
// ─────────────────────────────────────────────────────────────────────────────

pub use aggregator::SignalAggregator;
pub use bus::{SignalReceiver, SignalSender, signal_channel};
pub use engine::SignalEngine;
pub use feature_store::{ComputedFeatures, FeatureStore};
pub use processor::SignalProcessor;
pub use processors::{MomentumProcessor, SmartMoneyProcessor, WhaleFlowProcessor};
pub use types::{
    Direction, FeatureVector, SignalEvent, SignalInput, SignalType, WhaleEvent,
};
