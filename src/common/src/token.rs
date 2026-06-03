//! Token identity primitives.

use crate::Pubkey;

/// Minimal token descriptor shared by pricing, routing, and graph layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    mint: Pubkey,
    decimals: u8,
}

impl Token {
    /// Creates a token descriptor.
    #[must_use]
    pub const fn new(mint: Pubkey, decimals: u8) -> Self {
        Self { mint, decimals }
    }

    /// Returns the token mint.
    #[must_use]
    pub const fn mint(&self) -> Pubkey {
        self.mint
    }

    /// Returns the token decimal precision.
    #[must_use]
    pub const fn decimals(&self) -> u8 {
        self.decimals
    }
}
