//! Strategy execution engine — arbitrage detection, publishers, and dispatcher.

#![forbid(unsafe_code)]

pub mod arbitrage;
pub mod dispatcher;
pub mod halt;
pub mod hotpath_runtime;
pub mod metrics;
pub mod publishers;

pub use dispatcher::{Signal, SignalBus, SignalPayload, StrategyDispatcher, StrategyMode};
pub use publishers::PublisherHandles;
