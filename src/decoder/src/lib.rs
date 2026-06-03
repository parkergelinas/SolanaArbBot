//! DEX decoding boundary.
//!
//! This crate converts raw RPC account bytes into structured pool state. The
//! current decoders intentionally use simplified placeholder layouts so the
//! abstraction can evolve before full Solana binary layouts are implemented.

#![forbid(unsafe_code)]

pub mod orca;
pub mod raydium;

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use common::{Result, Token};
use tracing::{debug, trace};

/// Supported DEX families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DexType {
    /// Raydium AMM-style pools.
    Raydium,
    /// Orca concentrated-liquidity pools.
    OrcaCLMM,
}

/// Structured pool state emitted by DEX decoders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolState {
    pub dex: DexType,
    pub token_a: Token,
    pub token_b: Token,
    pub liquidity: u128,
    pub reserves: Option<(u64, u64)>,
}

/// Shared decoder interface for pool-state decoders.
pub trait PoolDecoder {
    /// Decodes raw RPC bytes into structured pool state.
    fn decode(&self, data: &[u8]) -> Result<PoolState>;
}

/// Unified decoder that routes to a DEX-specific decoder by `DexType`.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnifiedDecoder {
    raydium: raydium::RaydiumDecoder,
    orca: orca::OrcaDecoder,
}

impl UnifiedDecoder {
    /// Creates a unified decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            raydium: raydium::RaydiumDecoder::new(),
            orca: orca::OrcaDecoder::new(),
        }
    }

    /// Decodes raw bytes using the requested DEX parser.
    pub fn decode(&self, dex: DexType, data: &[u8]) -> Result<PoolState> {
        let stage_started_at = Instant::now();
        let event_timestamp_micros = unix_timestamp_micros();
        trace!(
            ?dex,
            len = data.len(),
            event_timestamp_micros,
            "decoder received pool bytes"
        );

        let result = match dex {
            DexType::Raydium => self.raydium.decode(data),
            DexType::OrcaCLMM => self.orca.decode(data),
        };

        debug!(
            ?dex,
            len = data.len(),
            event_timestamp_micros,
            latency_micros = stage_started_at.elapsed().as_micros(),
            success = result.is_ok(),
            "decoder stage complete"
        );

        result
    }
}

/// Backwards-compatible alias for existing scaffold code.
pub type DexDecoder = UnifiedDecoder;

fn unix_timestamp_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::{raydium, DexType, UnifiedDecoder};
    use crate::orca;

    #[test]
    fn unified_decoder_routes_raydium() {
        let state = UnifiedDecoder::new()
            .decode(DexType::Raydium, &raydium::tests::raydium_fixture())
            .expect("decode raydium");

        assert_eq!(state.dex, DexType::Raydium);
        assert_eq!(state.reserves, Some((1_000, 2_000)));
    }

    #[test]
    fn unified_decoder_routes_orca_clmm() {
        let state = UnifiedDecoder::new()
            .decode(DexType::OrcaCLMM, &orca::tests::orca_fixture())
            .expect("decode orca");

        assert_eq!(state.dex, DexType::OrcaCLMM);
        assert_eq!(state.reserves, None);
    }
}
