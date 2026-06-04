//! Market event types passed between ingestion, decoding, and analysis stages.

use super::pubkey::Pubkey;

/// Expandable market event emitted by ingestion and decoding stages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarketEvent {
    /// Pool state changed.
    PoolUpdate(PoolUpdate),
    /// Swap activity was observed.
    SwapEvent(SwapEvent),
    /// Concentrated-liquidity tick state changed.
    TickUpdate(TickUpdate),
}

/// Generic pool state update shared across DEX decoders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolUpdate {
    pub pool: Option<Pubkey>,
    pub token_a_mint: Option<Pubkey>,
    pub token_b_mint: Option<Pubkey>,
    pub liquidity: Option<u128>,
    pub sqrt_price: Option<u128>,
    pub fee_rate: Option<u64>,
}

/// Generic swap event shared across execution and pricing layers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapEvent {
    pub pool: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub amount_in: u128,
    pub amount_out: u128,
}

/// Generic tick update shared by concentrated-liquidity decoders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickUpdate {
    pub pool: Pubkey,
    pub tick_index: i32,
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
}
