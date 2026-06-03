//! Raydium account decoders.

use common::{RaydiumAmmV4Pool, Result};

use crate::layout::{read_pubkey, read_u64};

const DECODER: &str = "raydium_amm_v4";

/// Raydium AMM v4 liquidity state account size.
pub const AMM_V4_ACCOUNT_LEN: usize = 752;

const STATUS_OFFSET: usize = 0;
const NONCE_OFFSET: usize = 8;
const BASE_DECIMAL_OFFSET: usize = 32;
const QUOTE_DECIMAL_OFFSET: usize = 40;
const TRADE_FEE_NUMERATOR_OFFSET: usize = 144;
const TRADE_FEE_DENOMINATOR_OFFSET: usize = 152;
const SWAP_FEE_NUMERATOR_OFFSET: usize = 176;
const SWAP_FEE_DENOMINATOR_OFFSET: usize = 184;
const BASE_VAULT_OFFSET: usize = 336;
const QUOTE_VAULT_OFFSET: usize = 368;
const BASE_MINT_OFFSET: usize = 400;
const QUOTE_MINT_OFFSET: usize = 432;
const LP_MINT_OFFSET: usize = 464;
const OPEN_ORDERS_OFFSET: usize = 496;
const MARKET_ID_OFFSET: usize = 528;
const MARKET_PROGRAM_ID_OFFSET: usize = 560;

/// Returns true when account bytes match the Raydium AMM v4 account length.
#[must_use]
pub fn is_raydium_amm_v4(data: &[u8]) -> bool {
    data.len() == AMM_V4_ACCOUNT_LEN
}

/// Decodes a Raydium AMM v4 liquidity state account.
pub fn decode_amm_v4(data: &[u8]) -> Result<RaydiumAmmV4Pool> {
    Ok(RaydiumAmmV4Pool {
        status: read_u64(data, STATUS_OFFSET, DECODER)?,
        nonce: read_u64(data, NONCE_OFFSET, DECODER)?,
        base_decimal: read_u64(data, BASE_DECIMAL_OFFSET, DECODER)?,
        quote_decimal: read_u64(data, QUOTE_DECIMAL_OFFSET, DECODER)?,
        trade_fee_numerator: read_u64(data, TRADE_FEE_NUMERATOR_OFFSET, DECODER)?,
        trade_fee_denominator: read_u64(data, TRADE_FEE_DENOMINATOR_OFFSET, DECODER)?,
        swap_fee_numerator: read_u64(data, SWAP_FEE_NUMERATOR_OFFSET, DECODER)?,
        swap_fee_denominator: read_u64(data, SWAP_FEE_DENOMINATOR_OFFSET, DECODER)?,
        base_vault: read_pubkey(data, BASE_VAULT_OFFSET, DECODER)?,
        quote_vault: read_pubkey(data, QUOTE_VAULT_OFFSET, DECODER)?,
        base_mint: read_pubkey(data, BASE_MINT_OFFSET, DECODER)?,
        quote_mint: read_pubkey(data, QUOTE_MINT_OFFSET, DECODER)?,
        lp_mint: read_pubkey(data, LP_MINT_OFFSET, DECODER)?,
        open_orders: read_pubkey(data, OPEN_ORDERS_OFFSET, DECODER)?,
        market_id: read_pubkey(data, MARKET_ID_OFFSET, DECODER)?,
        market_program_id: read_pubkey(data, MARKET_PROGRAM_ID_OFFSET, DECODER)?,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use common::{Error, Pubkey};

    pub(crate) fn amm_v4_fixture() -> Vec<u8> {
        let mut data = vec![0; AMM_V4_ACCOUNT_LEN];
        write_u64(&mut data, STATUS_OFFSET, 6);
        write_u64(&mut data, NONCE_OFFSET, 255);
        write_u64(&mut data, BASE_DECIMAL_OFFSET, 9);
        write_u64(&mut data, QUOTE_DECIMAL_OFFSET, 6);
        write_u64(&mut data, TRADE_FEE_NUMERATOR_OFFSET, 25);
        write_u64(&mut data, TRADE_FEE_DENOMINATOR_OFFSET, 10_000);
        write_u64(&mut data, SWAP_FEE_NUMERATOR_OFFSET, 25);
        write_u64(&mut data, SWAP_FEE_DENOMINATOR_OFFSET, 10_000);
        write_pubkey(&mut data, BASE_VAULT_OFFSET, Pubkey::new([1; 32]));
        write_pubkey(&mut data, QUOTE_VAULT_OFFSET, Pubkey::new([2; 32]));
        write_pubkey(&mut data, BASE_MINT_OFFSET, Pubkey::new([3; 32]));
        write_pubkey(&mut data, QUOTE_MINT_OFFSET, Pubkey::new([4; 32]));
        write_pubkey(&mut data, LP_MINT_OFFSET, Pubkey::new([5; 32]));
        write_pubkey(&mut data, OPEN_ORDERS_OFFSET, Pubkey::new([6; 32]));
        write_pubkey(&mut data, MARKET_ID_OFFSET, Pubkey::new([7; 32]));
        write_pubkey(&mut data, MARKET_PROGRAM_ID_OFFSET, Pubkey::new([8; 32]));
        data
    }

    #[test]
    fn detects_raydium_amm_v4_by_size() {
        assert!(is_raydium_amm_v4(&amm_v4_fixture()));
        assert!(!is_raydium_amm_v4(&[]));
    }

    #[test]
    fn decodes_raydium_amm_v4_pool_state() {
        let pool = decode_amm_v4(&amm_v4_fixture()).expect("decode raydium");

        assert_eq!(pool.status, 6);
        assert_eq!(pool.nonce, 255);
        assert_eq!(pool.base_decimal, 9);
        assert_eq!(pool.quote_decimal, 6);
        assert_eq!(pool.trade_fee_denominator, 10_000);
        assert_eq!(pool.base_vault, Pubkey::new([1; 32]));
        assert_eq!(pool.quote_mint, Pubkey::new([4; 32]));
        assert_eq!(pool.market_program_id, Pubkey::new([8; 32]));
    }

    #[test]
    fn rejects_short_raydium_amm_v4_input() {
        let err = decode_amm_v4(&[0; 16]).expect_err("short input");

        assert!(matches!(
            err,
            Error::DecodeInputTooShort {
                decoder: "raydium_amm_v4",
                ..
            }
        ));
    }

    fn write_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_pubkey(data: &mut [u8], offset: usize, value: Pubkey) {
        data[offset..offset + 32].copy_from_slice(value.as_bytes());
    }
}
