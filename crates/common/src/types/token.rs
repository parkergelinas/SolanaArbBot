//! Token identity primitives.

use super::pubkey::Pubkey;

/// Minimal token descriptor shared by pricing, routing, and graph layers.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Token {
    mint: Pubkey,
    decimals: u8,
    symbol: Option<String>,
}

impl Token {
    /// Creates a token descriptor.
    #[must_use]
    pub fn new(mint: Pubkey, decimals: u8, symbol: Option<String>) -> Self {
        Self {
            mint,
            decimals,
            symbol,
        }
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

    /// Returns the optional token symbol.
    #[must_use]
    pub fn symbol(&self) -> Option<&str> {
        self.symbol.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::Token;
    use crate::Pubkey;

    #[test]
    fn token_keeps_identity_fields() {
        let mint = Pubkey::new([3; 32]);
        let token = Token::new(mint, 9, Some("SOL".to_owned()));

        assert_eq!(token.mint(), mint);
        assert_eq!(token.decimals(), 9);
        assert_eq!(token.symbol(), Some("SOL"));
    }

    #[test]
    fn token_allows_missing_symbol() {
        let token = Token::new(Pubkey::default(), 6, None);

        assert_eq!(token.symbol(), None);
    }
}
