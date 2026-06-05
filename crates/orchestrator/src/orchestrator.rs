use std::sync::Arc;

use config::SystemConfig;
use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::{
    coordinator::{SubsystemId, SubsystemRegistry},
    error::{OrchestratorError, OrchestratorResult},
    gate::{ExecutionApprovalRequest, ExecutionGate, GateDecision},
    regression::{MetricsSnapshot, RegressionChecker, RegressionReport, RegressionThresholds},
    state::{HealthStatus, RiskState, RunMode, StateHandle, SystemState},
    validator::ArchitectureValidator,
};

// ─────────────────────────────────────────────────────────────────────────────
// Orchestrator
// ─────────────────────────────────────────────────────────────────────────────

/// Central governance and orchestration layer.
///
/// Owns the [`StateHandle`], the [`ExecutionGate`], the subsystem registry,
/// and the regression checker.  All state transitions go through here; no
/// subsystem mutates global state directly.
pub struct Orchestrator {
    pub(crate) state_handle: Arc<StateHandle>,
    gate: Arc<ExecutionGate>,
    registry: Arc<SubsystemRegistry>,
    #[allow(dead_code)]
    validator: ArchitectureValidator,
    regression: RegressionChecker,
    config: Arc<SystemConfig>,
}

impl Orchestrator {
    /// Construct the orchestrator.
    ///
    /// Returns `(orchestrator, state_receiver)`.  The receiver reflects every
    /// state transition going forward and can be shared with subsystems.
    pub fn new(config: Arc<SystemConfig>) -> (Self, watch::Receiver<SystemState>) {
        let initial = SystemState::default();
        let (state_handle, receiver) = StateHandle::new(initial);
        let state_handle = Arc::new(state_handle);

        let gate_rx = state_handle.subscribe();
        let gate = Arc::new(ExecutionGate::new(gate_rx, Arc::clone(&config)));
        let registry = Arc::new(SubsystemRegistry::new());

        let thresholds = RegressionThresholds {
            max_pnl_degradation_pct: config.orchestrator.regression_pnl_threshold_pct,
            max_drawdown_increase_pct: config.orchestrator.regression_drawdown_threshold_pct,
            max_win_rate_drop_pct: config.orchestrator.regression_win_rate_threshold_pct,
            max_slippage_increase_pct: 20.0,
        };

        let orch = Self {
            state_handle,
            gate,
            registry,
            validator: ArchitectureValidator,
            regression: RegressionChecker::new(thresholds),
            config,
        };

        (orch, receiver)
    }

    // ─────────────────────────────────────────────────────────────────────
    // Startup
    // ─────────────────────────────────────────────────────────────────────

    /// Validate architecture before any subsystem starts.
    ///
    /// Warnings are logged; errors cause an `Err(ArchViolation)` return.
    pub fn validate_startup(&self) -> OrchestratorResult<()> {
        match ArchitectureValidator::run_all(&self.config) {
            Ok(warnings) => {
                for w in &warnings {
                    warn!(rule = w.rule, description = %w.description, "arch warning");
                }
                Ok(())
            }
            Err(errors) => {
                let msgs: Vec<String> = errors.iter().map(|v| v.description.clone()).collect();
                Err(OrchestratorError::ArchViolation(msgs.join("; ")))
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // Trading state
    // ─────────────────────────────────────────────────────────────────────

    /// Enable trading.
    ///
    /// In paper mode this always succeeds (assuming risk is not locked).
    /// In live mode `config.features.enable_live_trading` must be `true`.
    pub fn enable_trading(&self, reason: &str) -> OrchestratorResult<()> {
        let current = self.state_handle.current();

        if current.risk_state == RiskState::Locked {
            return Err(OrchestratorError::InvalidTransition {
                from: format!("{:?}", current.risk_state),
                to: "trading_enabled=true".to_string(),
                reason: "cannot enable trading while risk state is Locked".to_string(),
            });
        }

        if current.mode == RunMode::Live && !self.config.features.enable_live_trading {
            return Err(OrchestratorError::InvalidTransition {
                from: format!("{:?}", current.mode),
                to: "trading_enabled=true".to_string(),
                reason: "live mode requires config.features.enable_live_trading = true".to_string(),
            });
        }

        let mut new_state = current;
        new_state.trading_enabled = true;
        new_state.reason = Some(reason.to_string());
        new_state.last_updated_micros = crate::now_micros();

        info!(reason, "trading enabled");
        self.state_handle.transition(new_state)
    }

    /// Emergency stop — immediately sets `trading_enabled = false` and
    /// `risk_state = Locked`.  Never returns an error.
    pub fn emergency_stop(&self, reason: &str) {
        let mut state = self.state_handle.current();
        state.trading_enabled = false;
        state.risk_state = RiskState::Locked;
        state.execution_active = false;
        state.reason = Some(format!("EMERGENCY STOP: {reason}"));
        state.last_updated_micros = crate::now_micros();
        let _ = self.state_handle.transition(state);
        error!(reason, "emergency stop triggered");
    }

    /// Transition the risk state.
    pub fn set_risk_state(&self, new_risk: RiskState, reason: &str) {
        let mut state = self.state_handle.current();
        state.risk_state = new_risk;
        state.reason = Some(reason.to_string());
        state.last_updated_micros = crate::now_micros();
        let _ = self.state_handle.transition(state);
    }

    /// Update the health status.
    ///
    /// Automatically triggers an emergency stop when `Critical` and
    /// `config.orchestrator.emergency_stop_on_critical_health` is `true`.
    pub fn update_health(&self, status: HealthStatus, reason: &str) {
        if status == HealthStatus::Critical
            && self.config.orchestrator.emergency_stop_on_critical_health
        {
            // Route through emergency_stop so the single transition is atomic.
            let mut state = self.state_handle.current();
            state.health_status = HealthStatus::Critical;
            state.trading_enabled = false;
            state.risk_state = RiskState::Locked;
            state.reason = Some(format!("CRITICAL HEALTH: {reason}"));
            state.last_updated_micros = crate::now_micros();
            let _ = self.state_handle.transition(state);
            error!(reason, "health critical — emergency stop");
        } else {
            let mut state = self.state_handle.current();
            state.health_status = status;
            state.reason = Some(reason.to_string());
            state.last_updated_micros = crate::now_micros();
            let _ = self.state_handle.transition(state);
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // Execution gate
    // ─────────────────────────────────────────────────────────────────────

    /// Evaluate an execution approval request through the gate.
    pub fn approve_execution(&self, req: &ExecutionApprovalRequest) -> GateDecision {
        self.gate.check(req)
    }

    // ─────────────────────────────────────────────────────────────────────
    // Subsystem lifecycle
    // ─────────────────────────────────────────────────────────────────────

    /// Notify the orchestrator that `id` has reached the `Running` state.
    pub fn subsystem_running(&self, id: SubsystemId) {
        self.registry.mark_running(id.clone());

        let mut state = self.state_handle.current();
        match id {
            SubsystemId::Ingestion => state.ingestion_active = true,
            SubsystemId::SignalEngine => state.signal_engine_active = true,
            SubsystemId::ExecutionEngine => state.execution_active = true,
            _ => {}
        }
        state.last_updated_micros = crate::now_micros();
        let _ = self.state_handle.transition(state);
    }

    /// Notify the orchestrator that `id` has failed with the given `reason`.
    /// Automatically degrades health if critical subsystems are affected.
    pub fn subsystem_failed(&self, id: SubsystemId, reason: &str) {
        self.registry.mark_failed(id, reason.to_string());

        if self.registry.all_critical_running() {
            self.update_health(HealthStatus::Degraded, &format!("subsystem degraded: {reason}"));
        } else {
            self.update_health(
                HealthStatus::Critical,
                &format!("critical subsystem failed: {reason}"),
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // Regression
    // ─────────────────────────────────────────────────────────────────────

    /// Compare metrics and return a regression report with a rollback
    /// suggestion if a major regression is detected.
    pub fn check_regression(
        &self,
        baseline: &MetricsSnapshot,
        current: &MetricsSnapshot,
    ) -> RegressionReport {
        self.regression.compare(baseline, current)
    }

    // ─────────────────────────────────────────────────────────────────────
    // Diagnostics
    // ─────────────────────────────────────────────────────────────────────

    /// One-line summary suitable for health-endpoint or structured logging.
    pub fn status_summary(&self) -> String {
        let s = self.state_handle.current();
        format!(
            "mode={:?} risk={:?} trading={} health={:?} reason={}",
            s.mode,
            s.risk_state,
            s.trading_enabled,
            s.health_status,
            s.reason.as_deref().unwrap_or("none"),
        )
    }
}
