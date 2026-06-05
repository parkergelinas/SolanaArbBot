//! Trade execution engine — async signal router, strategies, Jupiter, risk, lifecycle.

#![forbid(unsafe_code)]

pub mod audit;
pub mod config;
pub mod confirmation;
pub mod jupiter;
pub mod latency;
pub mod orders;
pub mod risk;
pub mod router;
pub mod signals;
pub mod strategy;
pub mod subscriber;

pub use config::EngineConfig;
pub use latency::LatencyBudget;
pub use orders::{OrderRecord, OrderStatus, OrderStore};
pub use router::ExecutionRouter;
pub use signals::TradeSignal;
