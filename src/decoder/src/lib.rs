//! DEX decoding boundary.
//!
//! This crate will decode Solana account and instruction data into market events.

#![forbid(unsafe_code)]

mod layout;
pub mod orca;
pub mod raydium;

pub mod dex {
    //! DEX-specific decoder entry points.

    use common::{MarketEvent, Result};
    use tracing::trace;

    use crate::{orca, raydium};

    /// Decoder for supported DEX account layouts.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct DexDecoder;

    impl DexDecoder {
        /// Creates a decoder.
        #[must_use]
        pub const fn new() -> Self {
            Self
        }

        /// Decodes a supported DEX account into a market event.
        pub fn decode(&self, data: &[u8]) -> Result<Option<MarketEvent>> {
            if raydium::is_raydium_amm_v4(data) {
                trace!(len = data.len(), "decoding raydium amm v4 account");
                return raydium::decode_amm_v4(data)
                    .map(|pool| Some(MarketEvent::RaydiumAmmV4(pool)));
            }

            if orca::is_orca_whirlpool(data) {
                trace!(len = data.len(), "decoding orca whirlpool account");
                return orca::decode_whirlpool(data)
                    .map(|pool| Some(MarketEvent::OrcaWhirlpool(pool)));
            }

            trace!(len = data.len(), "unsupported dex account layout");
            Ok(None)
        }
    }
}

pub use dex::DexDecoder;

#[cfg(test)]
mod tests {
    use super::{orca, raydium, DexDecoder};
    use common::MarketEvent;

    #[test]
    fn decoder_returns_none_for_unknown_layout() {
        assert!(DexDecoder::new().decode(&[]).expect("decode").is_none());
    }

    #[test]
    fn decoder_dispatches_raydium_layout() {
        let data = raydium::tests::amm_v4_fixture();
        let event = DexDecoder::new()
            .decode(&data)
            .expect("decode")
            .expect("event");

        assert!(matches!(event, MarketEvent::RaydiumAmmV4(_)));
    }

    #[test]
    fn decoder_dispatches_orca_layout() {
        let data = orca::tests::whirlpool_fixture();
        let event = DexDecoder::new()
            .decode(&data)
            .expect("decode")
            .expect("event");

        assert!(matches!(event, MarketEvent::OrcaWhirlpool(_)));
    }
}
