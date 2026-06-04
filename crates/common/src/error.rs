//! Workspace-wide error types.

use thiserror::Error;

/// Common error variants shared by infrastructure crates.
#[derive(Debug, Error)]
pub enum Error {
    /// RPC transport or request failure.
    #[error("rpc error: {0}")]
    RpcError(String),

    /// Decoder failure while parsing external data.
    #[error("decode error: {0}")]
    DecodeError(String),

    /// State failed validation or an invariant check.
    #[error("invalid state: {0}")]
    InvalidState(String),

    /// Internal system error.
    #[error("internal error: {0}")]
    InternalError(String),
}

/// Workspace result alias.
pub type Result<T> = std::result::Result<T, Error>;
