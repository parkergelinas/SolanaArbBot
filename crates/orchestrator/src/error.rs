use crate::gate::GateRejectionReason;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("architecture violation: {0}")]
    ArchViolation(String),

    #[error("invalid state transition from {from:?} to {to:?}: {reason}")]
    InvalidTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("execution blocked: {0:?}")]
    ExecutionBlocked(Vec<GateRejectionReason>),

    #[error("config error: {0}")]
    Config(String),
}

pub type OrchestratorResult<T> = Result<T, OrchestratorError>;
