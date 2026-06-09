//! Orca Whirlpool (CLMM) account decoder.
//!
//! Program ID: `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`
//!
//! # Whirlpool account layout (little-endian, offsets relative to byte 0)
//!
//! | Offset | Size | Field              |
//! |--------|------|--------------------|
//! | 65     | 16   | sqrt_price (u128)  |
//! | 81     | 16   | liquidity (u128)   |
//! | 97     | 4    | tick_current_index (i32) |
//! | 101    | 2    | fee_rate (u16)     |
//! | 103    | 32   | token_mint_a (Pubkey) |
//! | 133    | 32   | token_mint_b (Pubkey) |
//!
//! Minimum account data length: 165 bytes.
//!
//! Price is derived as: `price_f64 = (sqrt_price as f64 / 2^64)^2`.

use common::{Error, MarketEvent, PoolUpdate, Pubkey, Result, Token};

use crate::{DexType, EventPoolDecoder, PoolDecoder, PoolState};

pub const WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

const SQRT_PRICE_OFFSET: usize = 65;
const LIQUIDITY_OFFSET: usize = 81;
const TICK_CURRENT_INDEX_OFFSET: usize = 97;
const FEE_RATE_OFFSET: usize = 101;
const TOKEN_MINT_A_OFFSET: usize = 103;
const TOKEN_MINT_B_OFFSET: usize = 133;
pub const WHIRLPOOL_MIN_DATA_LEN: usize = TOKEN_MINT_B_OFFSET + 32; // 165

const DEFAULT_DECIMALS: u8 = 0;
const DECODER: &str = "orca_whirlpool";

/// Orca Whirlpool CLMM pool decoder.
#[derive(Clone, Copy, Debug, Default)]
pub struct OrcaDecoder;

impl OrcaDecoder {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PoolDecoder for OrcaDecoder {
    fn decode(&self, data: &[u8]) -> Result<PoolState> {
        ensure_len(data, WHIRLPOOL_MIN_DATA_LEN)?;

        let sqrt_price = read_u128(data, SQRT_PRICE_OFFSET)?;
        let liquidity = read_u128(data, LIQUIDITY_OFFSET)?;
        let _tick_current_index = read_i32(data, TICK_CURRENT_INDEX_OFFSET)?;
        let _fee_rate = read_u16(data, FEE_RATE_OFFSET)?;
        let token_mint_a = read_pubkey(data, TOKEN_MINT_A_OFFSET)?;
        let token_mint_b = read_pubkey(data, TOKEN_MINT_B_OFFSET)?;

        // Convert sqrt_price → a (reserve_a, reserve_b) proxy.
        // price = (sqrt_price / 2^64)^2; store as integer ratio scaled to 1e6.
        let reserves = sqrt_price_to_reserves(sqrt_price);

        Ok(PoolState {
            dex: DexType::OrcaCLMM,
            token_a: Token::new(token_mint_a, DEFAULT_DECIMALS, None),
            token_b: Token::new(token_mint_b, DEFAULT_DECIMALS, None),
            liquidity,
            reserves: Some(reserves),
        })
    }
}

impl EventPoolDecoder for OrcaDecoder {
    fn decode_event(&self, event: &MarketEvent) -> Result<PoolState> {
        let MarketEvent::PoolUpdate(update) = event else {
            return Err(Error::DecodeError(
                "orca decoder only transforms pool update events".to_owned(),
            ));
        };
        pool_update_to_state(update)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Converts Whirlpool sqrt_price to an (reserve_a, reserve_b) proxy suitable
/// for the `PoolState::reserves` field.  The ratio represents the exchange rate
/// scaled by 1_000_000.
fn sqrt_price_to_reserves(sqrt_price: u128) -> (u64, u64) {
    // price = (sqrt_price / 2^64)^2
    // Avoid f64 inf for huge values; clamp to u64::MAX.
    let q64 = (1u128 << 64) as f64;
    let sp_f = sqrt_price as f64 / q64;
    let price = sp_f * sp_f;

    let reserve_b = 1_000_000u64;
    let reserve_a = if price > 0.0 && price.is_finite() {
        ((reserve_b as f64) / price).min(u64::MAX as f64) as u64
    } else {
        reserve_b
    };
    (reserve_a, reserve_b)
}

fn pool_update_to_state(update: &PoolUpdate) -> Result<PoolState> {
    let token_a_mint = required_pubkey(update.token_a_mint, "orca token_a_mint")?;
    let token_b_mint = required_pubkey(update.token_b_mint, "orca token_b_mint")?;
    let liquidity = update
        .liquidity
        .ok_or_else(|| Error::DecodeError("orca pool update missing liquidity".to_owned()))?;

    let reserve_a = u64::try_from(liquidity).unwrap_or(u64::MAX);
    let reserve_b = update
        .sqrt_price
        .and_then(|sp| u64::try_from(sp.saturating_sub(1_000_000) / 10).ok())
        .filter(|r| *r > 0)
        .unwrap_or(reserve_a);

    Ok(PoolState {
        dex: DexType::OrcaCLMM,
        token_a: Token::new(token_a_mint, DEFAULT_DECIMALS, None),
        token_b: Token::new(token_b_mint, DEFAULT_DECIMALS, None),
        liquidity,
        reserves: Some((reserve_a, reserve_b)),
    })
}

fn required_pubkey(value: Option<Pubkey>, field: &'static str) -> Result<Pubkey> {
    value.ok_or_else(|| Error::DecodeError(format!("{field} is required")))
}

fn ensure_len(data: &[u8], expected: usize) -> Result<()> {
    if data.len() < expected {
        return Err(Error::DecodeError(format!(
            "{DECODER} pool data too short: expected at least {expected} bytes, got {}",
            data.len()
        )));
    }
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let mut bytes = [0; 32];
    bytes.copy_from_slice(read_slice(data, offset, 32)?);
    Ok(Pubkey::new(bytes))
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let mut bytes = [0; 2];
    bytes.copy_from_slice(read_slice(data, offset, 2)?);
    Ok(u16::from_le_bytes(bytes))
}

fn read_i32(data: &[u8], offset: usize) -> Result<i32> {
    let mut bytes = [0; 4];
    bytes.copy_from_slice(read_slice(data, offset, 4)?);
    Ok(i32::from_le_bytes(bytes))
}

fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    let mut bytes = [0; 16];
    bytes.copy_from_slice(read_slice(data, offset, 16)?);
    Ok(u128::from_le_bytes(bytes))
}

fn read_slice(data: &[u8], offset: usize, len: usize) -> Result<&[u8]> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| Error::DecodeError(format!("{DECODER} offset overflow")))?;
    data.get(offset..end).ok_or_else(|| {
        Error::DecodeError(format!(
            "{DECODER} pool data too short: need {end} bytes, got {}",
            data.len()
        ))
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a hardcoded 165-byte Whirlpool account snapshot with known field values.
    pub(crate) fn orca_fixture() -> Vec<u8> {
        let mut data = vec![0u8; WHIRLPOOL_MIN_DATA_LEN];

        // sqrt_price at offset 65 — encode 2^64 (price = 1.0) as u128 LE
        let sqrt_price: u128 = 1u128 << 64;
        data[SQRT_PRICE_OFFSET..SQRT_PRICE_OFFSET + 16]
            .copy_from_slice(&sqrt_price.to_le_bytes());

        // liquidity at offset 81
        let liquidity: u128 = 500_000_000;
        data[LIQUIDITY_OFFSET..LIQUIDITY_OFFSET + 16]
            .copy_from_slice(&liquidity.to_le_bytes());

        // tick_current_index at offset 97 (-100i32)
        data[TICK_CURRENT_INDEX_OFFSET..TICK_CURRENT_INDEX_OFFSET + 4]
            .copy_from_slice(&(-100i32).to_le_bytes());

        // fee_rate at offset 101 (300 = 0.3%)
        data[FEE_RATE_OFFSET..FEE_RATE_OFFSET + 2]
            .copy_from_slice(&300u16.to_le_bytes());

        // token_mint_a at offset 103
        let mint_a = Pubkey::new([0xAAu8; 32]);
        data[TOKEN_MINT_A_OFFSET..TOKEN_MINT_A_OFFSET + 32]
            .copy_from_slice(mint_a.as_bytes());

        // token_mint_b at offset 133
        let mint_b = Pubkey::new([0xBBu8; 32]);
        data[TOKEN_MINT_B_OFFSET..TOKEN_MINT_B_OFFSET + 32]
            .copy_from_slice(mint_b.as_bytes());

        data
    }

    #[test]
    fn decodes_whirlpool_snapshot() {
        let state = OrcaDecoder::new()
            .decode(&orca_fixture())
            .expect("decode whirlpool");

        assert_eq!(state.dex, DexType::OrcaCLMM);
        assert_eq!(state.token_a.mint(), Pubkey::new([0xAAu8; 32]));
        assert_eq!(state.token_b.mint(), Pubkey::new([0xBBu8; 32]));
        assert_eq!(state.liquidity, 500_000_000);
        // price = 1.0 → reserves should be equal
        let (ra, rb) = state.reserves.expect("reserves present");
        assert_eq!(ra, rb, "price=1.0 must produce equal reserve proxy");
    }

    #[test]
    fn rejects_short_whirlpool_data() {
        let err = OrcaDecoder::new().decode(&[0u8; 8]).expect_err("too short");
        assert!(matches!(err, Error::DecodeError(_)));
    }

    #[test]
    fn transforms_orca_pool_update_event() {
        let event = MarketEvent::PoolUpdate(PoolUpdate {
            pool: Some(Pubkey::new([9; 32])),
            token_a_mint: Some(Pubkey::new([3; 32])),
            token_b_mint: Some(Pubkey::new([4; 32])),
            liquidity: Some(7_000),
            sqrt_price: Some(123),
            fee_rate: Some(30),
        });

        let state = OrcaDecoder::new()
            .decode_event(&event)
            .expect("transform orca");

        assert_eq!(state.dex, DexType::OrcaCLMM);
        assert_eq!(state.token_a.mint(), Pubkey::new([3; 32]));
        assert_eq!(state.token_b.mint(), Pubkey::new([4; 32]));
        assert_eq!(state.liquidity, 7_000);
    }

    #[test]
    fn rejects_orca_event_with_missing_liquidity() {
        let event = MarketEvent::PoolUpdate(PoolUpdate {
            pool: None,
            token_a_mint: Some(Pubkey::new([3; 32])),
            token_b_mint: Some(Pubkey::new([4; 32])),
            liquidity: None,
            sqrt_price: None,
            fee_rate: None,
        });
        let err = OrcaDecoder::new()
            .decode_event(&event)
            .expect_err("missing liquidity");
        assert!(matches!(err, Error::DecodeError(_)));
    }

    #[tokio::test]
    async fn sign_and_decode_round_trip_is_deterministic() {
        // Verify determinism: decoding the same bytes twice yields equal states.
        let data = orca_fixture();
        let decoder = OrcaDecoder::new();
        let s1 = decoder.decode(&data).unwrap();
        let s2 = decoder.decode(&data).unwrap();
        assert_eq!(s1, s2);
    }
}
