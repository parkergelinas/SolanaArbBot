use thiserror::Error;

#[derive(Debug, Error)]
pub enum RiskError {
    #[error("risk limit exceeded: {0}")]
    LimitExceeded(&'static str),

    #[error("invalid risk configuration: {0}")]
    InvalidConfig(&'static str),
}
