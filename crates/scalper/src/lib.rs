//! Modular Solana DEX scalping engine — paper simulation only.
//!
//! # Architecture
//!
//! ```text
//! SignalEvent
//!   │
//!   ▼
//! ScalpEngine::evaluate
//!   ├─ signal expiry check
//!   ├─ sizing::compute_size
//!   ├─ microstructure::round_trip_cost_bps
//!   ├─ TradeCandidate
//!   ├─ TradeFilterChain::check_all (6 filters)
//!   │     LiquidityFilter → VolatilityRegimeFilter → SlippageFilter
//!   │     → CooldownFilter → RateLimitFilter → MinEdgeFilter
//!   └─ PaperSimulator::simulate_fill → TradeResult
//!                                         │
//!                                         ▼
//!                                 trade_history → PnlSummary
//! ```
//!
//! # Guarantees
//!
//! - **No execution logic**: `dry_run` is always `true` for this crate.
//! - **No live RPC calls**: all data comes from function parameters.
//! - **No hardcoded thresholds**: every value comes from [`ScalerConfig`].
//! - **No `Arc<RwLock<>>`** in the hot path: filters use `DashMap`.

#![forbid(unsafe_code)]

pub mod candidate;
pub mod config;
pub mod engine;
pub mod filters;
pub mod microstructure;
pub mod pnl;
pub mod simulator;
pub mod sizing;

// ─────────────────────────────────────────────────────────────────────────────
// Public API surface
// ─────────────────────────────────────────────────────────────────────────────

pub use candidate::{TradeCandidate, TradeResult};
pub use ::config::ScalerConfig;
pub use engine::ScalpEngine;
pub use filters::{FilterResult, TradeFilter, TradeFilterChain};
pub use microstructure::{
    jupiter_effective_fee, raydium_price_impact, round_trip_cost_bps, whirlpool_price_impact,
};
pub use pnl::PnlSummary;
pub use simulator::PaperSimulator;
pub use sizing::compute_size;
