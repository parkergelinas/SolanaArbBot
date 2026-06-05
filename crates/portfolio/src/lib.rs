//! Portfolio management boundary.
//!
//! Canonical home for:
//! - [`portfolio_risk`]: portfolio-level risk state machine (drawdown, daily/monthly limits)
//! - [`position_sizing`]: position sizing with win/loss streak adjustments
//!
//! The legacy `risk_engine` and `position` workspace crates are thin re-export
//! wrappers over this crate for backward compatibility.

#![forbid(unsafe_code)]

pub mod portfolio_risk;
pub mod position_sizing;

pub use portfolio_risk::{evaluate_limits, RiskAction, RiskConfig, RiskState};
pub use position_sizing::compute_position_size;
