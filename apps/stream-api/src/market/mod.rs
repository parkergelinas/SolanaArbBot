//! In-memory market state — DashMap-backed, lock-free hot path reads.

mod engine;
mod state;

pub use engine::{spawn_market_engine, MarketEngineHandle};
pub use state::{CandleKey, PoolSnapshot};
