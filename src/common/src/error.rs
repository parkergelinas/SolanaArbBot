//! Workspace-wide error types.

use thiserror::Error;

/// Common error variants shared by infrastructure crates.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// Placeholder for APIs that are intentionally scaffolded only.
    #[error("operation is not implemented: {0}")]
    NotImplemented(&'static str),

    /// Input or decoded data failed validation.
    #[error("invalid data: {0}")]
    InvalidData(&'static str),
}

/// Workspace result alias.
pub type Result<T> = std::result::Result<T, Error>;
