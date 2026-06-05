//! Market event sources for autonomous operation (synthetic + external buffers).

use std::collections::VecDeque;

use common::{MarketEvent, PoolUpdate, Pubkey, SwapEvent};
use decoder::DexType;
use signals::WhaleEvent;

pub struct PairPool {
    pub pool: Pubkey,
    pub dex: DexType,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub base_liquidity: u128,
    pub base_reserve_b: u64,
    pub tvl_usd: f64,
    pub pair_id: u8,
}

pub fn default_pools() -> Vec<PairPool> {
    let mut pools = Vec::new();
    for pair in 0u8..5 {
        let liq = 2_000_000u128 + u128::from(pair) * 500_000;
        let res_b = 6_700u64 + u64::from(pair) * 500;
        pools.push(PairPool {
            pool: pool_key(pair, 0),
            dex: DexType::Raydium,
            token_a: token_mint(pair, 0),
            token_b: token_mint(pair, 1),
            base_liquidity: liq,
            base_reserve_b: res_b,
            tvl_usd: 800_000.0 + f64::from(pair) * 100_000.0,
            pair_id: pair,
        });
        pools.push(PairPool {
            pool: pool_key(pair, 1),
            dex: DexType::OrcaCLMM,
            token_a: token_mint(pair, 0),
            token_b: token_mint(pair, 1),
            base_liquidity: liq + 200_000,
            base_reserve_b: res_b,
            tvl_usd: 900_000.0 + f64::from(pair) * 100_000.0,
            pair_id: pair,
        });
    }
    pools
}

fn pool_key(pair: u8, dex: u8) -> Pubkey {
    Pubkey::new([pair, dex, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
}

fn token_mint(pair: u8, leg: u8) -> Pubkey {
    Pubkey::new([pair, leg, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1])
}

/// One tick of synthetic events (pool updates + swaps + optional whale).
pub fn tick_events(step: u64, ts_micros: u64) -> Vec<(u64, MarketEvent)> {
    let pools = default_pools();
    let phase = (step % 120) as f64;
    let mut out = Vec::new();

    for spec in &pools {
        let noise = ((step + u64::from(spec.pair_id) * 7) % 17) as f64 * 0.0003;
        let momentum = (phase / 120.0).sin() * 0.008;
        let mut reserve_b = spec.base_reserve_b as f64;
        reserve_b *= 1.0 + momentum + noise;
        if spec.dex == DexType::OrcaCLMM && step % 90 < 8 {
            reserve_b *= 1.040;
        }
        let reserve_b = reserve_b.max(100.0) as u64;
        let liquidity = spec.base_liquidity + u128::from(step % 50) * 1_000;

        out.push((
            ts_micros,
            MarketEvent::PoolUpdate(PoolUpdate {
                pool: Some(spec.pool),
                token_a_mint: Some(spec.token_a),
                token_b_mint: Some(spec.token_b),
                liquidity: Some(liquidity),
                sqrt_price: Some(1_000_000 + u128::from(reserve_b) * 10),
                fee_rate: Some(25),
            }),
        ));

        if step % 3 == u64::from(spec.pair_id) % 3 {
            out.push((
                ts_micros + 50_000,
                MarketEvent::SwapEvent(SwapEvent {
                    pool: spec.pool,
                    input_mint: spec.token_a,
                    output_mint: spec.token_b,
                    amount_in: 50_000u128 + u128::from(step % 20) * 5_000,
                    amount_out: 49_500u128 + u128::from(step % 20) * 4_900,
                }),
            ));
        }
    }

    out
}

/// Bounded buffer for signal-bus / intelligence-api events (no synthetic fallback).
#[derive(Clone, Debug, Default)]
pub struct ExternalIngestionBuffer {
    market: VecDeque<(u64, MarketEvent)>,
    whales: VecDeque<(u64, WhaleEvent)>,
}

impl ExternalIngestionBuffer {
    pub fn push_market(&mut self, ts_micros: u64, event: MarketEvent) {
        self.market.push_back((ts_micros, event));
    }

    pub fn push_whale(&mut self, ts_micros: u64, event: WhaleEvent) {
        self.whales.push_back((ts_micros, event));
    }

    pub fn drain_market(&mut self) -> Vec<(u64, MarketEvent)> {
        self.market.drain(..).collect()
    }

    pub fn drain_whales(&mut self) -> Vec<(u64, WhaleEvent)> {
        self.whales.drain(..).collect()
    }
}
