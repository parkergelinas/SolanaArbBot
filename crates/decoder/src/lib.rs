//! DEX decoding boundary.
//!
//! This crate converts raw RPC account bytes into structured pool state. The
//! current decoders intentionally use simplified placeholder layouts so the
//! abstraction can evolve before full Solana binary layouts are implemented.

#![forbid(unsafe_code)]

pub mod meteora;
pub mod orca_clmm;
pub use orca_clmm as orca;
pub mod raydium;

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use common::{MarketEvent, Result, Token};
use tracing::{debug, trace};

/// Supported DEX families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DexType {
    /// Raydium AMM-style pools.
    Raydium,
    /// Orca concentrated-liquidity pools.
    OrcaCLMM,
    /// Meteora Dynamic Liquidity Market Maker pools.
    MeteoraDLMM,
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

/// Shared decoder interface for market-event transformations.
pub trait EventPoolDecoder {
    /// Converts a market event into normalized pool state.
    fn decode_event(&self, event: &MarketEvent) -> Result<PoolState>;
}

/// Unified decoder that routes to a DEX-specific decoder by `DexType`.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnifiedDecoder {
    raydium: raydium::RaydiumDecoder,
    orca: orca::OrcaDecoder,
    meteora: meteora::MeteoraDecoder,
}

impl UnifiedDecoder {
    /// Creates a unified decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            raydium: raydium::RaydiumDecoder::new(),
            orca: orca::OrcaDecoder::new(),
            meteora: meteora::MeteoraDecoder::default(),
        }
    }

    /// Creates a decoder with a configurable SOL/USD price for Meteora TVL estimates.
    #[must_use]
    pub fn with_sol_price(sol_price_usd: f64) -> Self {
        Self {
            raydium: raydium::RaydiumDecoder::new(),
            orca: orca::OrcaDecoder::new(),
            meteora: meteora::MeteoraDecoder::new(sol_price_usd),
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
            DexType::MeteoraDLMM => self.meteora.decode(data),
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

    /// Converts a generic market event into normalized pool state for the requested DEX.
    pub fn decode_event(&self, dex: DexType, event: &MarketEvent) -> Result<PoolState> {
        let stage_started_at = Instant::now();
        let event_timestamp_micros = unix_timestamp_micros();
        trace!(
            ?dex,
            ?event,
            event_timestamp_micros,
            "decoder received market event"
        );

        let result = match dex {
            DexType::Raydium => self.raydium.decode_event(event),
            DexType::OrcaCLMM => self.orca.decode_event(event),
            // Meteora DLMM uses account-data decoding; event-based transform not supported.
            DexType::MeteoraDLMM => Err(common::Error::DecodeError(
                "Meteora DLMM does not support event-based decoding; use decode() with raw account bytes".into(),
            )),
        };

        debug!(
            ?dex,
            event_timestamp_micros,
            latency_micros = stage_started_at.elapsed().as_micros(),
            success = result.is_ok(),
            "decoder event transform complete"
        );

        result
    }
}

/// Backwards-compatible alias for existing scaffold code.
pub type DexDecoder = UnifiedDecoder;

/// Convenience free function: decode Meteora DLMM account bytes with a given SOL price.
pub fn decode_meteora_dlmm(data: &[u8], sol_price_usd: f64) -> Result<PoolState> {
    meteora::MeteoraDecoder::new(sol_price_usd).decode(data)
}

fn unix_timestamp_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::{meteora, raydium, DexType, UnifiedDecoder};
    use crate::orca;
    use common::{MarketEvent, PoolUpdate, Pubkey};

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
        // Whirlpool decoder always computes a reserve proxy from sqrt_price.
        assert!(state.reserves.is_some());
    }

    #[test]
    fn unified_decoder_transforms_raydium_market_event() {
        let event = pool_update_event(10_000);
        let state = UnifiedDecoder::new()
            .decode_event(DexType::Raydium, &event)
            .expect("transform raydium event");

        assert_eq!(state.dex, DexType::Raydium);
        assert_eq!(state.token_a.mint(), Pubkey::new([1; 32]));
        assert_eq!(state.token_b.mint(), Pubkey::new([2; 32]));
        assert_eq!(state.liquidity, 10_000);
        assert_eq!(state.reserves, Some((10_000, 10_000)));
    }

    #[test]
    fn unified_decoder_transforms_orca_market_event() {
        let event = pool_update_event(20_000);
        let state = UnifiedDecoder::new()
            .decode_event(DexType::OrcaCLMM, &event)
            .expect("transform orca event");

        assert_eq!(state.dex, DexType::OrcaCLMM);
        assert_eq!(state.token_a.mint(), Pubkey::new([1; 32]));
        assert_eq!(state.token_b.mint(), Pubkey::new([2; 32]));
        assert_eq!(state.liquidity, 20_000);
        assert_eq!(state.reserves, Some((20_000, 20_000)));
    }

    #[test]
    fn event_transform_is_deterministic() {
        let event = pool_update_event(42_000);
        let decoder = UnifiedDecoder::new();

        let first = decoder
            .decode_event(DexType::Raydium, &event)
            .expect("first transform");
        let second = decoder
            .decode_event(DexType::Raydium, &event)
            .expect("second transform");

        assert_eq!(first, second);
    }

    #[test]
    fn unified_decoder_routes_meteora_dlmm() {
        let state = UnifiedDecoder::new()
            .decode(DexType::MeteoraDLMM, &meteora::tests::meteora_fixture())
            .expect("decode meteora");

        assert_eq!(state.dex, DexType::MeteoraDLMM);
        assert_eq!(state.reserves, Some((2_000_000, 3_000_000)));
        assert_eq!(state.liquidity, 5_000_000);
    }

    #[tokio::test]
    async fn decode_meteora_dlmm_free_function() {
        let state = super::decode_meteora_dlmm(&meteora::tests::meteora_fixture(), 150.0)
            .expect("decode via free fn");
        assert_eq!(state.dex, DexType::MeteoraDLMM);
    }

    fn pool_update_event(liquidity: u128) -> MarketEvent {
        MarketEvent::PoolUpdate(PoolUpdate {
            pool: Some(Pubkey::new([9; 32])),
            token_a_mint: Some(Pubkey::new([1; 32])),
            token_b_mint: Some(Pubkey::new([2; 32])),
            liquidity: Some(liquidity),
            sqrt_price: Some(1_000),
            fee_rate: Some(25),
        })
    }
}
