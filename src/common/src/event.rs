//! Market event types passed between ingestion, decoding, and analysis stages.

use crate::Pubkey;

/// Decoded market event emitted by DEX decoders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketEvent {
    /// Raydium AMM v4 pool account state.
    RaydiumAmmV4(RaydiumAmmV4Pool),
    /// Orca Whirlpool account state.
    OrcaWhirlpool(OrcaWhirlpoolPool),
}

/// Raydium AMM v4 pool account snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RaydiumAmmV4Pool {
    pub status: u64,
    pub nonce: u64,
    pub base_decimal: u64,
    pub quote_decimal: u64,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market_id: Pubkey,
    pub market_program_id: Pubkey,
}

/// Orca Whirlpool account snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrcaWhirlpoolPool {
    pub whirlpools_config: Pubkey,
    pub tick_spacing: u16,
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub tick_current_index: i32,
    pub token_mint_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub fee_growth_global_a: u128,
    pub fee_growth_global_b: u128,
}
