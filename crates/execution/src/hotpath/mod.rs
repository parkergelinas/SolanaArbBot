//! Ultra low-latency synchronous execution pipeline.
//!
//! # Architecture
//!
//! ```text
//! MarketTick → FeatureUpdate → SignalEval → RiskGate → ExecutionIntent
//!      ↑ synchronous, stack-only, no logging, no JSON, no async
//! ```
//!
//! Network I/O (Jito / RPC) is delegated to a **cold-path** thread via a
//! preallocated SPSC queue.  The hot loop never blocks on I/O.

pub mod engine;
pub mod execute;
pub mod precompute;
pub mod risk;
pub mod signal;
pub mod state;
pub mod types;

pub use engine::{HotPathEngine, HotPathStats};
pub use execute::{ColdPathExecutor, ExecutionRouter, RouteChoice};
pub use precompute::PrecomputeTable;
pub use state::HotState;
pub use types::{
    ExecutionIntent, HotSignal, MarketTick, PoolIdx, RiskVerdict, TickOutcome, Venue,
};
