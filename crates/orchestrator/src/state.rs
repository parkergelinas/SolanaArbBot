use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::error::{OrchestratorError, OrchestratorResult};

// ─────────────────────────────────────────────────────────────────────────────
// Enums
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunMode {
    Paper,
    Backtest,
    /// Requires explicit unlock via `config.features.enable_live_trading`.
    Live,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskState {
    Normal,
    /// Position sizing is halved.
    Reduced,
    /// No new trades permitted.
    Locked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Ok,
    /// Some subsystems are warning.
    Degraded,
    /// Stop trading immediately.
    Critical,
}

// ─────────────────────────────────────────────────────────────────────────────
// SystemState
// ─────────────────────────────────────────────────────────────────────────────

/// Single source of truth for system-wide runtime state.
///
/// All subsystems read from this; only the orchestrator writes to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub mode: RunMode,
    pub risk_state: RiskState,
    /// Must be explicitly set to `true` — always starts `false`.
    pub trading_enabled: bool,
    pub health_status: HealthStatus,
    pub ingestion_active: bool,
    pub signal_engine_active: bool,
    pub execution_active: bool,
    pub last_updated_micros: u64,
    /// Human-readable explanation for the current state.
    pub reason: Option<String>,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            mode: RunMode::Paper,
            risk_state: RiskState::Normal,
            trading_enabled: false,
            health_status: HealthStatus::Ok,
            ingestion_active: false,
            signal_engine_active: false,
            execution_active: false,
            last_updated_micros: 0,
            reason: Some("system initializing".to_string()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// StateHandle
// ─────────────────────────────────────────────────────────────────────────────

/// Wraps a `watch` channel, broadcasting state changes to all subscribers.
///
/// Only the orchestrator calls [`StateHandle::transition`].
/// Subsystems subscribe with [`StateHandle::subscribe`] and receive the latest
/// state immediately on first borrow.
pub struct StateHandle {
    sender: watch::Sender<SystemState>,
}

impl StateHandle {
    /// Create a new handle pre-seeded with `initial`.
    /// Returns the handle and the *primary* receiver for the caller.
    pub fn new(initial: SystemState) -> (Self, watch::Receiver<SystemState>) {
        let (sender, receiver) = watch::channel(initial);
        (Self { sender }, receiver)
    }

    /// Transition system state. Only the orchestrator calls this.
    pub fn transition(&self, new_state: SystemState) -> OrchestratorResult<()> {
        self.sender.send(new_state).map_err(|_| OrchestratorError::InvalidTransition {
            from: "current".to_string(),
            to: "new".to_string(),
            reason: "all watch receivers have been dropped".to_string(),
        })
    }

    /// Read the current state without subscribing (non-blocking).
    pub fn current(&self) -> SystemState {
        self.sender.borrow().clone()
    }

    /// Subscribe — receive the latest state immediately plus every future
    /// transition.
    pub fn subscribe(&self) -> watch::Receiver<SystemState> {
        self.sender.subscribe()
    }
}
