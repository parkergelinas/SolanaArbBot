use std::sync::Arc;

use config::SystemConfig;
use tokio::sync::watch;

use crate::state::{HealthStatus, RiskState, RunMode, SystemState};

// ─────────────────────────────────────────────────────────────────────────────
// Request / decision types
// ─────────────────────────────────────────────────────────────────────────────

/// Approval request submitted by the execution layer before any trade.
pub struct ExecutionApprovalRequest {
    pub signal_id: u64,
    pub pool_address: String,
    pub trade_size_usd: f64,
    pub estimated_cost_bps: f64,
    pub net_edge_bps: f64,
    pub requested_at_micros: u64,
}

/// Result of gate evaluation.
pub struct GateDecision {
    pub approved: bool,
    /// Empty on the approved path — no heap allocation occurs.
    pub reasons: Vec<GateRejectionReason>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GateRejectionReason {
    /// `trading_enabled` is `false` — system is not in a runnable state.
    SystemNotRunning,
    /// Trading has been explicitly disabled.
    TradingDisabled,
    /// `RiskState::Locked` — all new trades are forbidden.
    RiskStateLocked,
    /// Trade size exceeds the reduced-risk cap (50 % of normal max).
    RiskStateReducedSizeExceeded { max_usd: f64 },
    /// `HealthStatus::Critical`.
    HealthCritical,
    /// System is in `Live` mode but `features.enable_live_trading` is `false`.
    LiveModeNotEnabled,
    /// Net edge is below the configured minimum.
    EdgeBelowMinimum { required_bps: f64, actual_bps: f64 },
    /// Trade notional exceeds the hard cap.
    TradeSizeExceedsLimit { max_usd: f64 },
}

// ─────────────────────────────────────────────────────────────────────────────
// ExecutionGate
// ─────────────────────────────────────────────────────────────────────────────

/// Final check before any trade action is permitted.
///
/// Hot-path — `check` must not allocate on the approved path.
pub struct ExecutionGate {
    state: watch::Receiver<SystemState>,
    config: Arc<SystemConfig>,
}

impl ExecutionGate {
    pub fn new(state: watch::Receiver<SystemState>, config: Arc<SystemConfig>) -> Self {
        Self { state, config }
    }

    /// Synchronous evaluation of all gate conditions in strict order.
    ///
    /// Returns [`GateDecision::approved = true`] only when every check passes.
    /// Collects all failing reasons (no short-circuit) so callers can log them.
    /// The returned `reasons` vec is empty on the approved path.
    pub fn check(&self, req: &ExecutionApprovalRequest) -> GateDecision {
        let state: SystemState = self.state.borrow().clone();

        // Accumulate failures; empty on the approved path (no allocation).
        let mut reasons: Vec<GateRejectionReason> = Vec::new();

        // 1. trading_enabled
        if !state.trading_enabled {
            reasons.push(GateRejectionReason::SystemNotRunning);
        }

        // 2. risk_state != Locked
        if state.risk_state == RiskState::Locked {
            reasons.push(GateRejectionReason::RiskStateLocked);
        }

        // 3. health_status != Critical
        if state.health_status == HealthStatus::Critical {
            reasons.push(GateRejectionReason::HealthCritical);
        }

        // 4. Live mode requires explicit unlock
        if state.mode == RunMode::Live && !self.config.features.enable_live_trading {
            reasons.push(GateRejectionReason::LiveModeNotEnabled);
        }

        // 5. net_edge_bps >= min_edge_bps
        let min_edge = self.config.scalper.min_edge_bps;
        if req.net_edge_bps < min_edge {
            reasons.push(GateRejectionReason::EdgeBelowMinimum {
                required_bps: min_edge,
                actual_bps: req.net_edge_bps,
            });
        }

        // 6. trade_size_usd <= max_trade_size_usd
        let max_size = self.config.execution.max_trade_size_usd;
        if req.trade_size_usd > max_size {
            reasons.push(GateRejectionReason::TradeSizeExceedsLimit { max_usd: max_size });
        }

        // 7. Reduced risk: trade_size_usd <= max * 0.5
        if state.risk_state == RiskState::Reduced {
            let reduced_max = max_size * 0.5;
            if req.trade_size_usd > reduced_max {
                reasons.push(GateRejectionReason::RiskStateReducedSizeExceeded {
                    max_usd: reduced_max,
                });
            }
        }

        GateDecision {
            approved: reasons.is_empty(),
            reasons,
        }
    }
}
