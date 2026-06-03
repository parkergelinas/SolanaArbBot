//! Lightweight public key wrapper used across the workspace.

/// Placeholder Solana public key representation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    /// Creates a public key wrapper from raw bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the raw 32-byte key.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
