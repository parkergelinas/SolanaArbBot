//! DEX decoding boundary.
//!
//! This crate will decode Solana account and instruction data into market events.

#![forbid(unsafe_code)]

pub mod dex {
    //! DEX-specific decoder entry points.

    use common::MarketEvent;

    /// Placeholder decoder for future Raydium and Orca CLMM support.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct DexDecoder;

    impl DexDecoder {
        /// Creates a placeholder decoder.
        #[must_use]
        pub const fn new() -> Self {
            Self
        }

        /// Returns no decoded events while the crate is scaffold-only.
        #[must_use]
        pub const fn decode(&self, _data: &[u8]) -> Option<MarketEvent> {
            None
        }
    }
}

pub use dex::DexDecoder;

#[cfg(test)]
mod tests {
    use super::DexDecoder;

    #[test]
    fn decoder_returns_no_events_in_scaffold() {
        assert!(DexDecoder::new().decode(&[]).is_none());
    }
}
