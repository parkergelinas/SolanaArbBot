//! Orca CLMM-style placeholder decoder.

use common::{Error, Pubkey, Result, Token};

use crate::{DexType, PoolDecoder, PoolState};

const DECODER: &str = "orca_clmm";
const TOKEN_A_MINT_OFFSET: usize = 0;
const TOKEN_B_MINT_OFFSET: usize = 32;
const TOKEN_A_DECIMALS_OFFSET: usize = 64;
const TOKEN_B_DECIMALS_OFFSET: usize = 65;
const LIQUIDITY_OFFSET: usize = 66;
const CURRENT_TICK_OFFSET: usize = 82;
const TICK_SPACING_OFFSET: usize = 86;

/// Simplified Orca CLMM placeholder layout length.
pub const ORCA_POOL_DATA_LEN: usize = 88;

/// Orca CLMM pool decoder.
#[derive(Clone, Copy, Debug, Default)]
pub struct OrcaDecoder;

impl OrcaDecoder {
    /// Creates an Orca decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PoolDecoder for OrcaDecoder {
    fn decode(&self, data: &[u8]) -> Result<PoolState> {
        ensure_len(data, ORCA_POOL_DATA_LEN)?;

        // Tick-aware fields are parsed now so the placeholder layout can grow
        // into a full CLMM representation without changing decoder routing.
        let _current_tick = read_i32(data, CURRENT_TICK_OFFSET)?;
        let _tick_spacing = read_u16(data, TICK_SPACING_OFFSET)?;

        Ok(PoolState {
            dex: DexType::OrcaCLMM,
            token_a: Token::new(
                read_pubkey(data, TOKEN_A_MINT_OFFSET)?,
                read_u8(data, TOKEN_A_DECIMALS_OFFSET)?,
                None,
            ),
            token_b: Token::new(
                read_pubkey(data, TOKEN_B_MINT_OFFSET)?,
                read_u8(data, TOKEN_B_DECIMALS_OFFSET)?,
                None,
            ),
            liquidity: read_u128(data, LIQUIDITY_OFFSET)?,
            reserves: None,
        })
    }
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

fn read_u8(data: &[u8], offset: usize) -> Result<u8> {
    Ok(*read_slice(data, offset, 1)?
        .first()
        .expect("slice length checked"))
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
            "{DECODER} pool data too short: expected at least {end} bytes, got {}",
            data.len()
        ))
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn orca_fixture() -> Vec<u8> {
        let mut data = vec![0; ORCA_POOL_DATA_LEN];
        write_pubkey(&mut data, TOKEN_A_MINT_OFFSET, Pubkey::new([3; 32]));
        write_pubkey(&mut data, TOKEN_B_MINT_OFFSET, Pubkey::new([4; 32]));
        data[TOKEN_A_DECIMALS_OFFSET] = 6;
        data[TOKEN_B_DECIMALS_OFFSET] = 9;
        write_u128(&mut data, LIQUIDITY_OFFSET, 123_456);
        write_i32(&mut data, CURRENT_TICK_OFFSET, -42);
        write_u16(&mut data, TICK_SPACING_OFFSET, 64);
        data
    }

    #[test]
    fn decodes_orca_pool_state() {
        let state = OrcaDecoder::new()
            .decode(&orca_fixture())
            .expect("decode orca");

        assert_eq!(state.dex, DexType::OrcaCLMM);
        assert_eq!(state.token_a.mint(), Pubkey::new([3; 32]));
        assert_eq!(state.token_a.decimals(), 6);
        assert_eq!(state.token_b.mint(), Pubkey::new([4; 32]));
        assert_eq!(state.token_b.decimals(), 9);
        assert_eq!(state.liquidity, 123_456);
        assert_eq!(state.reserves, None);
    }

    #[test]
    fn rejects_short_orca_pool_data() {
        let err = OrcaDecoder::new().decode(&[0; 8]).expect_err("short");

        assert!(matches!(err, Error::DecodeError(_)));
    }

    fn write_pubkey(data: &mut [u8], offset: usize, value: Pubkey) {
        data[offset..offset + 32].copy_from_slice(value.as_bytes());
    }

    fn write_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u128(data: &mut [u8], offset: usize, value: u128) {
        data[offset..offset + 16].copy_from_slice(&value.to_le_bytes());
    }
}
