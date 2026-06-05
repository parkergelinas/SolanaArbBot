//! Trade execution engine — signal router, strategies, Jupiter executor, order lifecycle.

#![forbid(unsafe_code)]

pub mod audit;
pub mod config;
pub mod jupiter;
pub mod orders;
pub mod router;
pub mod signals;
pub mod strategy;
pub mod subscriber;

pub use config::EngineConfig;
pub use orders::{OrderRecord, OrderStatus, OrderStore};
pub use router::ExecutionRouter;
pub use signals::TradeSignal;
