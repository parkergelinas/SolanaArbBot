//! Workspace-wide error types.

use thiserror::Error;

/// Common error variants shared by infrastructure crates.
#[derive(Debug, Error)]
pub enum Error {
    /// Placeholder for APIs that are intentionally scaffolded only.
    #[error("operation is not implemented: {0}")]
    NotImplemented(&'static str),

    /// Input or decoded data failed validation.
    #[error("invalid data: {0}")]
    InvalidData(&'static str),

    /// A byte slice did not contain exactly 32 bytes for a Solana public key.
    #[error("invalid pubkey length: expected 32 bytes, got {actual}")]
    InvalidPubkeyLength { actual: usize },

    /// A base58 public key string could not be decoded.
    #[error("invalid pubkey encoding")]
    InvalidPubkeyEncoding {
        #[source]
        source: bs58::decode::Error,
    },

    /// Token decimals exceeded the workspace sanity limit.
    #[error("invalid token decimals: {decimals} exceeds max {max}")]
    InvalidTokenDecimals { decimals: u8, max: u8 },

    /// Stream ingestion configuration was invalid.
    #[error("invalid stream config: {0}")]
    InvalidStreamConfig(&'static str),

    /// Stream ingestion queue is full.
    #[error("stream queue is full")]
    StreamQueueFull,

    /// Stream ingestion queue has closed.
    #[error("stream queue is closed")]
    StreamClosed,

    /// A decoder received fewer bytes than required for a known account layout.
    #[error("{decoder} decode input too short: expected at least {expected} bytes, got {actual}")]
    DecodeInputTooShort {
        decoder: &'static str,
        expected: usize,
        actual: usize,
    },

    /// A decoder received bytes that failed layout validation.
    #[error("{decoder} decode input invalid: {reason}")]
    DecodeInputInvalid {
        decoder: &'static str,
        reason: &'static str,
    },
}

/// Workspace result alias.
pub type Result<T> = std::result::Result<T, Error>;
