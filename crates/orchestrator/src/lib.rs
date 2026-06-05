//! Central governance and orchestration layer for the Solana trading system.
//!
//! [`Orchestrator`] is the single authority for system-wide state transitions,
//! subsystem lifecycle tracking, execution gating, and regression protection.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use config::SystemConfig;
//! use orchestrator::{Orchestrator, ExecutionApprovalRequest};
//!
//! let config = Arc::new(SystemConfig::default());
//! let (orch, _rx) = Orchestrator::new(config);
//!
//! orch.validate_startup().expect("arch validation passed");
//! orch.enable_trading("initial start").expect("paper mode enabled");
//!
//! let req = ExecutionApprovalRequest {
//!     signal_id: 1,
//!     pool_address: "pool123".to_string(),
//!     trade_size_usd: 10.0,
//!     estimated_cost_bps: 2.0,
//!     net_edge_bps: 30.0,
//!     requested_at_micros: 0,
//! };
//! let decision = orch.approve_execution(&req);
//! assert!(decision.approved);
//! ```

#![forbid(unsafe_code)]

pub mod coordinator;
pub mod error;
pub mod gate;
pub mod orchestrator;
pub mod regression;
pub mod state;
pub mod validator;

// ─────────────────────────────────────────────────────────────────────────────
// Flat re-exports
// ─────────────────────────────────────────────────────────────────────────────

pub use coordinator::{
    SubsystemDependency, SubsystemId, SubsystemRegistry, SubsystemStatus,
    canonical_dependency_graph, validate_startup_sequence,
};
pub use error::{OrchestratorError, OrchestratorResult};
pub use gate::{ExecutionApprovalRequest, ExecutionGate, GateDecision, GateRejectionReason};
pub use orchestrator::Orchestrator;
pub use regression::{
    MetricsSnapshot, RegressionChecker, RegressionItem, RegressionReport, RegressionSeverity,
    RegressionThresholds, RegressionVerdict,
};
pub use state::{HealthStatus, RiskState, RunMode, StateHandle, SystemState};
pub use validator::{ArchViolation, ArchitectureValidator, ViolationSeverity};

// ─────────────────────────────────────────────────────────────────────────────
// Internal utility
// ─────────────────────────────────────────────────────────────────────────────

/// Returns current Unix time in microseconds.
pub(crate) fn now_micros() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}
