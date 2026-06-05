//! Error types for the wallet crate.

#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("wallet operations are disabled in paper mode")]
    PaperMode,

    #[error("keypair file not found: {0}")]
    FileNotFound(String),

    #[error("invalid keypair format: {0}")]
    InvalidKeyFormat(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("network mismatch: expected {expected:?}, detected {detected:?} at {endpoint}")]
    NetworkMismatch {
        expected: String,
        detected: String,
        endpoint: String,
    },

    #[error("insufficient balance: have {have_sol:.4} SOL, need {need_sol:.4} SOL")]
    InsufficientBalance { have_sol: f64, need_sol: f64 },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type WalletResult<T> = Result<T, WalletError>;
