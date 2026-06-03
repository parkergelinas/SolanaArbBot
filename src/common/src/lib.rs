//! Shared primitives for the Solana market analysis workspace.
//!
//! This crate owns cross-cutting types that other crates can depend on without
//! introducing cycles.

#![forbid(unsafe_code)]

pub mod error;
pub mod event;
pub mod pubkey;
pub mod token;

pub use error::{Error, Result};
pub use event::{MarketEvent, PoolUpdate, SwapEvent, TickUpdate};
pub use pubkey::{Pubkey, PUBKEY_BYTES};
pub use token::Token;

#[cfg(test)]
mod tests {
    use super::{Pubkey, Token};

    #[test]
    fn token_keeps_core_identity_fields() {
        let mint = Pubkey::new([7; 32]);
        let token = Token::new(mint, 6, Some("USDC".to_owned()));

        assert_eq!(token.mint(), mint);
        assert_eq!(token.decimals(), 6);
        assert_eq!(token.symbol(), Some("USDC"));
    }
}
