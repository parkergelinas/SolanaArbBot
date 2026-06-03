//! Orca account decoders.

use common::{Error, OrcaWhirlpoolPool, Result};

use crate::layout::{read_i32, read_pubkey, read_u128, read_u16};

const DECODER: &str = "orca_whirlpool";

/// Anchor discriminator for `account:Whirlpool`.
pub const WHIRLPOOL_DISCRIMINATOR: [u8; 8] = [63, 149, 209, 12, 225, 128, 99, 9];

/// Orca Whirlpool account size.
pub const WHIRLPOOL_ACCOUNT_LEN: usize = 653;

const DISCRIMINATOR_OFFSET: usize = 0;
const WHIRLPOOLS_CONFIG_OFFSET: usize = 8;
const TICK_SPACING_OFFSET: usize = 41;
const FEE_RATE_OFFSET: usize = 45;
const PROTOCOL_FEE_RATE_OFFSET: usize = 47;
const LIQUIDITY_OFFSET: usize = 49;
const SQRT_PRICE_OFFSET: usize = 65;
const TICK_CURRENT_INDEX_OFFSET: usize = 81;
const TOKEN_MINT_A_OFFSET: usize = 101;
const TOKEN_VAULT_A_OFFSET: usize = 133;
const FEE_GROWTH_GLOBAL_A_OFFSET: usize = 165;
const TOKEN_MINT_B_OFFSET: usize = 181;
const TOKEN_VAULT_B_OFFSET: usize = 213;
const FEE_GROWTH_GLOBAL_B_OFFSET: usize = 245;

/// Returns true when account bytes match the Orca Whirlpool discriminator and size.
#[must_use]
pub fn is_orca_whirlpool(data: &[u8]) -> bool {
    data.len() == WHIRLPOOL_ACCOUNT_LEN
        && data.get(DISCRIMINATOR_OFFSET..DISCRIMINATOR_OFFSET + WHIRLPOOL_DISCRIMINATOR.len())
            == Some(WHIRLPOOL_DISCRIMINATOR.as_slice())
}

/// Decodes an Orca Whirlpool account.
pub fn decode_whirlpool(data: &[u8]) -> Result<OrcaWhirlpoolPool> {
    if data.len() < WHIRLPOOL_ACCOUNT_LEN {
        return Err(Error::DecodeInputTooShort {
            decoder: DECODER,
            expected: WHIRLPOOL_ACCOUNT_LEN,
            actual: data.len(),
        });
    }

    if !is_orca_whirlpool(data) {
        return Err(Error::DecodeInputInvalid {
            decoder: DECODER,
            reason: "invalid whirlpool discriminator or account length",
        });
    }

    Ok(OrcaWhirlpoolPool {
        whirlpools_config: read_pubkey(data, WHIRLPOOLS_CONFIG_OFFSET, DECODER)?,
        tick_spacing: read_u16(data, TICK_SPACING_OFFSET, DECODER)?,
        fee_rate: read_u16(data, FEE_RATE_OFFSET, DECODER)?,
        protocol_fee_rate: read_u16(data, PROTOCOL_FEE_RATE_OFFSET, DECODER)?,
        liquidity: read_u128(data, LIQUIDITY_OFFSET, DECODER)?,
        sqrt_price: read_u128(data, SQRT_PRICE_OFFSET, DECODER)?,
        tick_current_index: read_i32(data, TICK_CURRENT_INDEX_OFFSET, DECODER)?,
        token_mint_a: read_pubkey(data, TOKEN_MINT_A_OFFSET, DECODER)?,
        token_vault_a: read_pubkey(data, TOKEN_VAULT_A_OFFSET, DECODER)?,
        token_mint_b: read_pubkey(data, TOKEN_MINT_B_OFFSET, DECODER)?,
        token_vault_b: read_pubkey(data, TOKEN_VAULT_B_OFFSET, DECODER)?,
        fee_growth_global_a: read_u128(data, FEE_GROWTH_GLOBAL_A_OFFSET, DECODER)?,
        fee_growth_global_b: read_u128(data, FEE_GROWTH_GLOBAL_B_OFFSET, DECODER)?,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use common::Pubkey;

    pub(crate) fn whirlpool_fixture() -> Vec<u8> {
        let mut data = vec![0; WHIRLPOOL_ACCOUNT_LEN];
        data[DISCRIMINATOR_OFFSET..DISCRIMINATOR_OFFSET + WHIRLPOOL_DISCRIMINATOR.len()]
            .copy_from_slice(&WHIRLPOOL_DISCRIMINATOR);
        write_pubkey(&mut data, WHIRLPOOLS_CONFIG_OFFSET, Pubkey::new([1; 32]));
        write_u16(&mut data, TICK_SPACING_OFFSET, 64);
        write_u16(&mut data, FEE_RATE_OFFSET, 300);
        write_u16(&mut data, PROTOCOL_FEE_RATE_OFFSET, 1_200);
        write_u128(&mut data, LIQUIDITY_OFFSET, 123_456);
        write_u128(&mut data, SQRT_PRICE_OFFSET, 987_654);
        write_i32(&mut data, TICK_CURRENT_INDEX_OFFSET, -12);
        write_pubkey(&mut data, TOKEN_MINT_A_OFFSET, Pubkey::new([2; 32]));
        write_pubkey(&mut data, TOKEN_VAULT_A_OFFSET, Pubkey::new([3; 32]));
        write_u128(&mut data, FEE_GROWTH_GLOBAL_A_OFFSET, 11);
        write_pubkey(&mut data, TOKEN_MINT_B_OFFSET, Pubkey::new([4; 32]));
        write_pubkey(&mut data, TOKEN_VAULT_B_OFFSET, Pubkey::new([5; 32]));
        write_u128(&mut data, FEE_GROWTH_GLOBAL_B_OFFSET, 22);
        data
    }

    #[test]
    fn detects_orca_whirlpool_by_discriminator_and_size() {
        assert!(is_orca_whirlpool(&whirlpool_fixture()));
        assert!(!is_orca_whirlpool(&[0; WHIRLPOOL_ACCOUNT_LEN]));
    }

    #[test]
    fn decodes_orca_whirlpool_state() {
        let pool = decode_whirlpool(&whirlpool_fixture()).expect("decode whirlpool");

        assert_eq!(pool.whirlpools_config, Pubkey::new([1; 32]));
        assert_eq!(pool.tick_spacing, 64);
        assert_eq!(pool.fee_rate, 300);
        assert_eq!(pool.protocol_fee_rate, 1_200);
        assert_eq!(pool.liquidity, 123_456);
        assert_eq!(pool.sqrt_price, 987_654);
        assert_eq!(pool.tick_current_index, -12);
        assert_eq!(pool.token_mint_a, Pubkey::new([2; 32]));
        assert_eq!(pool.token_vault_b, Pubkey::new([5; 32]));
        assert_eq!(pool.fee_growth_global_b, 22);
    }

    #[test]
    fn rejects_short_orca_whirlpool_input() {
        let err = decode_whirlpool(&[0; 16]).expect_err("short input");

        assert!(matches!(
            err,
            Error::DecodeInputTooShort {
                decoder: "orca_whirlpool",
                expected: WHIRLPOOL_ACCOUNT_LEN,
                actual: 16
            }
        ));
    }

    #[test]
    fn rejects_invalid_orca_whirlpool_discriminator() {
        let err = decode_whirlpool(&[0; WHIRLPOOL_ACCOUNT_LEN]).expect_err("invalid input");

        assert!(matches!(
            err,
            Error::DecodeInputInvalid {
                decoder: "orca_whirlpool",
                ..
            }
        ));
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

    fn write_pubkey(data: &mut [u8], offset: usize, value: Pubkey) {
        data[offset..offset + 32].copy_from_slice(value.as_bytes());
    }
}
