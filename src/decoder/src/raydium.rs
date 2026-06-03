//! Raydium AMM-style placeholder decoder.

use common::{Error, MarketEvent, PoolUpdate, Pubkey, Result, Token};

use crate::{DexType, EventPoolDecoder, PoolDecoder, PoolState};

const DECODER: &str = "raydium";
const TOKEN_A_MINT_OFFSET: usize = 0;
const TOKEN_B_MINT_OFFSET: usize = 32;
const TOKEN_A_DECIMALS_OFFSET: usize = 64;
const TOKEN_B_DECIMALS_OFFSET: usize = 65;
const LIQUIDITY_OFFSET: usize = 66;
const RESERVE_A_OFFSET: usize = 82;
const RESERVE_B_OFFSET: usize = 90;

/// Simplified Raydium placeholder layout length.
pub const RAYDIUM_POOL_DATA_LEN: usize = 98;
const DEFAULT_EVENT_DECIMALS: u8 = 0;

/// Raydium AMM-style pool decoder.
#[derive(Clone, Copy, Debug, Default)]
pub struct RaydiumDecoder;

impl RaydiumDecoder {
    /// Creates a Raydium decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PoolDecoder for RaydiumDecoder {
    fn decode(&self, data: &[u8]) -> Result<PoolState> {
        ensure_len(data, RAYDIUM_POOL_DATA_LEN)?;

        let token_a = Token::new(
            read_pubkey(data, TOKEN_A_MINT_OFFSET)?,
            read_u8(data, TOKEN_A_DECIMALS_OFFSET)?,
            None,
        );
        let token_b = Token::new(
            read_pubkey(data, TOKEN_B_MINT_OFFSET)?,
            read_u8(data, TOKEN_B_DECIMALS_OFFSET)?,
            None,
        );

        Ok(PoolState {
            dex: DexType::Raydium,
            token_a,
            token_b,
            liquidity: read_u128(data, LIQUIDITY_OFFSET)?,
            reserves: Some((
                read_u64(data, RESERVE_A_OFFSET)?,
                read_u64(data, RESERVE_B_OFFSET)?,
            )),
        })
    }
}

impl EventPoolDecoder for RaydiumDecoder {
    fn decode_event(&self, event: &MarketEvent) -> Result<PoolState> {
        let MarketEvent::PoolUpdate(update) = event else {
            return Err(Error::DecodeError(
                "raydium decoder only transforms pool update events".to_owned(),
            ));
        };

        pool_update_to_state(update)
    }
}

fn pool_update_to_state(update: &PoolUpdate) -> Result<PoolState> {
    let token_a_mint = required_pubkey(update.token_a_mint, "raydium token_a_mint")?;
    let token_b_mint = required_pubkey(update.token_b_mint, "raydium token_b_mint")?;
    let liquidity = update
        .liquidity
        .ok_or_else(|| Error::DecodeError("raydium pool update missing liquidity".to_owned()))?;
    let reserve = u64::try_from(liquidity).unwrap_or(u64::MAX);

    Ok(PoolState {
        dex: DexType::Raydium,
        token_a: Token::new(token_a_mint, DEFAULT_EVENT_DECIMALS, None),
        token_b: Token::new(token_b_mint, DEFAULT_EVENT_DECIMALS, None),
        liquidity,
        reserves: Some((reserve, reserve)),
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

fn read_u8(data: &[u8], offset: usize) -> Result<u8> {
    Ok(*read_slice(data, offset, 1)?
        .first()
        .expect("slice length checked"))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(read_slice(data, offset, 8)?);
    Ok(u64::from_le_bytes(bytes))
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

    pub(crate) fn raydium_fixture() -> Vec<u8> {
        let mut data = vec![0; RAYDIUM_POOL_DATA_LEN];
        write_pubkey(&mut data, TOKEN_A_MINT_OFFSET, Pubkey::new([1; 32]));
        write_pubkey(&mut data, TOKEN_B_MINT_OFFSET, Pubkey::new([2; 32]));
        data[TOKEN_A_DECIMALS_OFFSET] = 9;
        data[TOKEN_B_DECIMALS_OFFSET] = 6;
        write_u128(&mut data, LIQUIDITY_OFFSET, 10_000);
        write_u64(&mut data, RESERVE_A_OFFSET, 1_000);
        write_u64(&mut data, RESERVE_B_OFFSET, 2_000);
        data
    }

    #[test]
    fn decodes_raydium_pool_state() {
        let state = RaydiumDecoder::new()
            .decode(&raydium_fixture())
            .expect("decode raydium");

        assert_eq!(state.dex, DexType::Raydium);
        assert_eq!(state.token_a.mint(), Pubkey::new([1; 32]));
        assert_eq!(state.token_a.decimals(), 9);
        assert_eq!(state.token_b.mint(), Pubkey::new([2; 32]));
        assert_eq!(state.token_b.decimals(), 6);
        assert_eq!(state.liquidity, 10_000);
        assert_eq!(state.reserves, Some((1_000, 2_000)));
    }

    #[test]
    fn rejects_short_raydium_pool_data() {
        let err = RaydiumDecoder::new().decode(&[0; 8]).expect_err("short");

        assert!(matches!(err, Error::DecodeError(_)));
    }

    #[test]
    fn transforms_raydium_pool_update_event() {
        let event = MarketEvent::PoolUpdate(PoolUpdate {
            pool: Some(Pubkey::new([9; 32])),
            token_a_mint: Some(Pubkey::new([1; 32])),
            token_b_mint: Some(Pubkey::new([2; 32])),
            liquidity: Some(5_000),
            sqrt_price: None,
            fee_rate: Some(25),
        });

        let state = RaydiumDecoder::new()
            .decode_event(&event)
            .expect("transform raydium");

        assert_eq!(state.dex, DexType::Raydium);
        assert_eq!(state.token_a.mint(), Pubkey::new([1; 32]));
        assert_eq!(state.token_a.decimals(), DEFAULT_EVENT_DECIMALS);
        assert_eq!(state.token_b.mint(), Pubkey::new([2; 32]));
        assert_eq!(state.liquidity, 5_000);
        assert_eq!(state.reserves, Some((5_000, 5_000)));
    }

    #[test]
    fn rejects_raydium_event_with_missing_token() {
        let event = MarketEvent::PoolUpdate(PoolUpdate {
            pool: None,
            token_a_mint: None,
            token_b_mint: Some(Pubkey::new([2; 32])),
            liquidity: Some(5_000),
            sqrt_price: None,
            fee_rate: None,
        });

        let err = RaydiumDecoder::new()
            .decode_event(&event)
            .expect_err("missing token");

        assert!(matches!(err, Error::DecodeError(_)));
    }

    fn write_pubkey(data: &mut [u8], offset: usize, value: Pubkey) {
        data[offset..offset + 32].copy_from_slice(value.as_bytes());
    }

    fn write_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u128(data: &mut [u8], offset: usize, value: u128) {
        data[offset..offset + 16].copy_from_slice(&value.to_le_bytes());
    }
}
