//! Token identity primitives.

use crate::{Error, Pubkey, Result};

/// Maximum token decimal precision accepted by the workspace.
pub const MAX_TOKEN_DECIMALS: u8 = 18;

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

    /// Creates a token descriptor after validating its decimal precision.
    pub const fn try_new(mint: Pubkey, decimals: u8) -> Result<Self> {
        if decimals > MAX_TOKEN_DECIMALS {
            return Err(Error::InvalidTokenDecimals {
                decimals,
                max: MAX_TOKEN_DECIMALS,
            });
        }

        Ok(Self::new(mint, decimals))
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

#[cfg(test)]
mod tests {
    use super::{Token, MAX_TOKEN_DECIMALS};
    use crate::{Error, Pubkey};

    #[test]
    fn token_try_new_accepts_valid_decimals() {
        let mint = Pubkey::new([3; 32]);
        let token = Token::try_new(mint, MAX_TOKEN_DECIMALS).expect("valid token");

        assert_eq!(token.mint(), mint);
        assert_eq!(token.decimals(), MAX_TOKEN_DECIMALS);
    }

    #[test]
    fn token_try_new_rejects_out_of_range_decimals() {
        let err = Token::try_new(Pubkey::default(), MAX_TOKEN_DECIMALS + 1).expect_err("invalid");

        assert!(matches!(
            err,
            Error::InvalidTokenDecimals {
                decimals: 19,
                max: MAX_TOKEN_DECIMALS
            }
        ));
    }
}
