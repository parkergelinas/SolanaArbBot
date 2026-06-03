//! Pricing engine boundary.
//!
//! This crate will convert decoded market events into normalized price inputs.

#![forbid(unsafe_code)]

pub mod engine {
    //! Price state update and query responsibilities.

    use common::Result;
    use decoder::DexDecoder;

    /// Placeholder pricing engine handle.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct PricingEngine {
        decoder: DexDecoder,
    }

    impl PricingEngine {
        /// Creates a placeholder pricing engine.
        #[must_use]
        pub const fn new(decoder: DexDecoder) -> Self {
            Self { decoder }
        }

        /// Returns the decoder boundary used by pricing.
        #[must_use]
        pub const fn decoder(&self) -> DexDecoder {
            self.decoder
        }

        /// Performs a no-op readiness check for the scaffold.
        pub const fn ready(&self) -> Result<()> {
            Ok(())
        }
    }
}

pub use engine::PricingEngine;

#[cfg(test)]
mod tests {
    use super::PricingEngine;
    use decoder::DexDecoder;

    #[test]
    fn pricing_engine_placeholder_is_ready() {
        let pricing = PricingEngine::new(DexDecoder::new());

        assert!(pricing.ready().is_ok());
        assert!(pricing.decoder().decode(&[]).expect("decode").is_none());
    }
}
