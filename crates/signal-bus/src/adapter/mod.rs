//! Source-specific parsers — kept separate from buffer business logic.

pub mod engine;
pub mod intelligence;
pub mod swap;

pub use engine::{engine_to_live_signal, EngineSignalInput};
pub use intelligence::{
    parse_batch_frame, smart_money_to_live_signal, whale_to_live_signal, RawSmartMoneyAlert,
    RawWhaleAlert,
};
pub use swap::{from_enriched, from_swap, SOL_MINT, USDC_MINT};
