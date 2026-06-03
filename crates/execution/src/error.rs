use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("execution planning failed: {0}")]
    Planning(String),

    #[error("transaction build failed: {0}")]
    TransactionBuild(String),
}
