//! Cross-DEX arbitrage detection: price streams → spread → ArbSignal → execution.

#![forbid(unsafe_code)]

pub mod config;
pub mod ingest;
pub mod latency;
pub mod normalizer;
pub mod pool_state;
pub mod router;
pub mod signal;
pub mod spread;
pub mod types;

pub use config::ArbConfig;
pub use router::ArbRouter;
pub use types::{ArbSignal, PoolPrice};
