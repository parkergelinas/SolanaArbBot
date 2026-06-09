//! Meteora DLMM (Dynamic Liquidity Market Maker) account decoder.
//!
//! Program ID: `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo`
//!
//! # Account layout (little-endian, offsets relative to byte 0)
//!
//! | Offset | Size | Field                  |
//! |--------|------|------------------------|
//! | 8      | 32   | token_x_mint (Pubkey)  |
//! | 40     | 32   | token_y_mint (Pubkey)  |
//! | 72     | 4    | active_bin_id (i32)    |
//! | 76     | 2    | bin_step (u16)         |
//! | 78     | 8    | reserve_x (u64)        |
//! | 86     | 8    | reserve_y (u64)        |
//!
//! Minimum account data length: 94 bytes.
//!
//! Mid-price: `price = (1.0 + bin_step as f64 / 10_000.0).powi(active_bin_id)`
//! Liquidity USD estimate: `(reserve_x + reserve_y) as f64 / 1e6 * sol_price`

use common::{Error, Pubkey, Result, Token};

use crate::{DexType, PoolDecoder, PoolState};

pub const METEORA_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

const TOKEN_X_MINT_OFFSET: usize = 8;
const TOKEN_Y_MINT_OFFSET: usize = 40;
const ACTIVE_BIN_ID_OFFSET: usize = 72;
const BIN_STEP_OFFSET: usize = 76;
const RESERVE_X_OFFSET: usize = 78;
const RESERVE_Y_OFFSET: usize = 86;
pub const METEORA_MIN_DATA_LEN: usize = RESERVE_Y_OFFSET + 8; // 94

const DECODER: &str = "meteora_dlmm";
const DEFAULT_DECIMALS: u8 = 0;

/// Meteora DLMM pool decoder.
///
/// Provide a current `sol_price_usd` so that `liquidity_usd` can be derived.
/// When `sol_price_usd` is not meaningful, pass `1.0`.
#[derive(Clone, Copy, Debug)]
pub struct MeteoraDecoder {
    pub sol_price_usd: f64,
}

impl Default for MeteoraDecoder {
    fn default() -> Self {
        Self { sol_price_usd: 1.0 }
    }
}

impl MeteoraDecoder {
    #[must_use]
    pub const fn new(sol_price_usd: f64) -> Self {
        Self { sol_price_usd }
    }

    /// Derives mid-price from bin parameters.
    pub fn mid_price(bin_step: u16, active_bin_id: i32) -> f64 {
        (1.0 + bin_step as f64 / 10_000.0).powi(active_bin_id)
    }

    /// Liquidity USD estimate from raw reserves.
    pub fn liquidity_usd(reserve_x: u64, reserve_y: u64, sol_price_usd: f64) -> f64 {
        (reserve_x as f64 + reserve_y as f64) / 1e6 * sol_price_usd
    }
}

impl PoolDecoder for MeteoraDecoder {
    fn decode(&self, data: &[u8]) -> Result<PoolState> {
        ensure_len(data, METEORA_MIN_DATA_LEN)?;

        let token_x_mint = read_pubkey(data, TOKEN_X_MINT_OFFSET)?;
        let token_y_mint = read_pubkey(data, TOKEN_Y_MINT_OFFSET)?;
        let active_bin_id = read_i32(data, ACTIVE_BIN_ID_OFFSET)?;
        let bin_step = read_u16(data, BIN_STEP_OFFSET)?;
        let reserve_x = read_u64(data, RESERVE_X_OFFSET)?;
        let reserve_y = read_u64(data, RESERVE_Y_OFFSET)?;

        let _mid_price = Self::mid_price(bin_step, active_bin_id);
        let _liquidity_usd = Self::liquidity_usd(reserve_x, reserve_y, self.sol_price_usd);

        let total_reserves = reserve_x.saturating_add(reserve_y);

        Ok(PoolState {
            dex: DexType::MeteoraDLMM,
            token_a: Token::new(token_x_mint, DEFAULT_DECIMALS, None),
            token_b: Token::new(token_y_mint, DEFAULT_DECIMALS, None),
            liquidity: total_reserves as u128,
            reserves: Some((reserve_x, reserve_y)),
        })
    }
}

// ── Low-level readers ────────────────────────────────────────────────────────

fn ensure_len(data: &[u8], expected: usize) -> Result<()> {
    if data.len() < expected {
        return Err(Error::DecodeError(format!(
            "{DECODER} account data too short: need {expected} bytes, got {}",
            data.len()
        )));
    }
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(read_slice(data, offset, 32)?);
    Ok(Pubkey::new(bytes))
}

fn read_i32(data: &[u8], offset: usize) -> Result<i32> {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(read_slice(data, offset, 4)?);
    Ok(i32::from_le_bytes(bytes))
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let mut bytes = [0u8; 2];
    bytes.copy_from_slice(read_slice(data, offset, 2)?);
    Ok(u16::from_le_bytes(bytes))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(read_slice(data, offset, 8)?);
    Ok(u64::from_le_bytes(bytes))
}

fn read_slice(data: &[u8], offset: usize, len: usize) -> Result<&[u8]> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| Error::DecodeError(format!("{DECODER} offset overflow")))?;
    data.get(offset..end).ok_or_else(|| {
        Error::DecodeError(format!(
            "{DECODER} account data too short: need {end} bytes, got {}",
            data.len()
        ))
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Hardcoded 94-byte Meteora DLMM account snapshot with known field values.
    pub(crate) fn meteora_fixture() -> Vec<u8> {
        let mut data = vec![0u8; METEORA_MIN_DATA_LEN];

        // token_x_mint at offset 8
        data[TOKEN_X_MINT_OFFSET..TOKEN_X_MINT_OFFSET + 32]
            .copy_from_slice(&[0x11u8; 32]);

        // token_y_mint at offset 40
        data[TOKEN_Y_MINT_OFFSET..TOKEN_Y_MINT_OFFSET + 32]
            .copy_from_slice(&[0x22u8; 32]);

        // active_bin_id at offset 72 (bin id = 100 → price ≈ 1.01^100)
        data[ACTIVE_BIN_ID_OFFSET..ACTIVE_BIN_ID_OFFSET + 4]
            .copy_from_slice(&100i32.to_le_bytes());

        // bin_step at offset 76 (1 basis point = 10_000 → 0.01% steps)
        data[BIN_STEP_OFFSET..BIN_STEP_OFFSET + 2]
            .copy_from_slice(&100u16.to_le_bytes());

        // reserve_x at offset 78
        data[RESERVE_X_OFFSET..RESERVE_X_OFFSET + 8]
            .copy_from_slice(&2_000_000u64.to_le_bytes());

        // reserve_y at offset 86
        data[RESERVE_Y_OFFSET..RESERVE_Y_OFFSET + 8]
            .copy_from_slice(&3_000_000u64.to_le_bytes());

        data
    }

    #[test]
    fn decodes_meteora_dlmm_snapshot() {
        let state = MeteoraDecoder::default()
            .decode(&meteora_fixture())
            .expect("decode meteora");

        assert_eq!(state.dex, DexType::MeteoraDLMM);
        assert_eq!(state.token_a.mint(), Pubkey::new([0x11u8; 32]));
        assert_eq!(state.token_b.mint(), Pubkey::new([0x22u8; 32]));
        assert_eq!(state.reserves, Some((2_000_000, 3_000_000)));
        assert_eq!(state.liquidity, 5_000_000);
    }

    #[test]
    fn mid_price_at_bin_zero_is_one() {
        let price = MeteoraDecoder::mid_price(100, 0);
        assert!((price - 1.0).abs() < 1e-12);
    }

    #[test]
    fn mid_price_increases_with_positive_bin_id() {
        let p_neg = MeteoraDecoder::mid_price(100, -50);
        let p_pos = MeteoraDecoder::mid_price(100, 50);
        assert!(p_pos > p_neg);
    }

    #[test]
    fn liquidity_usd_scales_with_sol_price() {
        let usd_1 = MeteoraDecoder::liquidity_usd(1_000_000, 1_000_000, 1.0);
        let usd_2 = MeteoraDecoder::liquidity_usd(1_000_000, 1_000_000, 2.0);
        assert!((usd_2 - 2.0 * usd_1).abs() < 1e-9);
    }

    #[test]
    fn rejects_short_data() {
        let err = MeteoraDecoder::default()
            .decode(&[0u8; 10])
            .expect_err("short");
        assert!(matches!(err, Error::DecodeError(_)));
    }

    #[tokio::test]
    async fn decode_is_deterministic() {
        let data = meteora_fixture();
        let dec = MeteoraDecoder::new(150.0);
        let s1 = dec.decode(&data).unwrap();
        let s2 = dec.decode(&data).unwrap();
        assert_eq!(s1, s2);
    }
}
