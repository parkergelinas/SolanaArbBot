//! Deterministic synthetic historical replay dataset.
//!
//! Models 25 pools across 5 token pairs on Raydium + Orca with:
//! - Momentum-friendly swap bursts (scalping)
//! - Periodic cross-DEX reserve skew (DEX-to-DEX arb)

use common::{MarketEvent, PoolUpdate, Pubkey, SwapEvent};
use decoder::DexType;
use signals::{Direction, SignalInput, WhaleEvent};

/// One timestamped replay event.
#[derive(Clone, Debug)]
pub enum ReplayEvent {
    Market(MarketEvent),
    Whale(WhaleEvent),
}


/// Ordered replay dataset with metadata.
#[derive(Clone, Debug)]
pub struct ReplayDataset {
    pub events: Vec<(u64, ReplayEvent)>,
    pub duration_secs: u64,
    pub pool_count: usize,
}

impl ReplayDataset {
    pub fn slice(&self, start: usize, end: usize) -> Self {
        let end = end.min(self.events.len());
        let start = start.min(end);
        Self {
            events: self.events[start..end].to_vec(),
            duration_secs: self.duration_secs,
            pool_count: self.pool_count,
        }
    }

    pub fn duration_hours(&self) -> f64 {
        self.duration_secs as f64 / 3600.0
    }
}

/// Pool metadata for synthetic generation.
struct PoolSpec {
    pool: Pubkey,
    dex: DexType,
    token_a: Pubkey,
    token_b: Pubkey,
    base_liquidity: u128,
    base_reserve_a: u64,
    base_reserve_b: u64,
    tvl_usd: f64,
    pair_id: u8,
}

fn pool_pubkey(pair: u8, dex: u8) -> Pubkey {
    Pubkey::new([pair, dex, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
}

fn token_mint(pair: u8, leg: u8) -> Pubkey {
    Pubkey::new([pair, leg, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1])
}

/// Generates a deterministic 24-hour dataset compressed to `duration_secs`.
///
/// `interval_secs` controls event cadence (default 8 s → ~10,800 events/day).
pub fn generate_dataset(duration_secs: u64, interval_secs: u64) -> ReplayDataset {
    let mut pools: Vec<PoolSpec> = Vec::new();

    for pair in 0u8..5 {
        let liq = 2_000_000u128 + u128::from(pair) * 500_000;
        let res_a = 1_000_000u64 + u64::from(pair) * 100_000;
        let res_b = 6_700u64 + u64::from(pair) * 500;

        pools.push(PoolSpec {
            pool: pool_pubkey(pair, 0),
            dex: DexType::Raydium,
            token_a: token_mint(pair, 0),
            token_b: token_mint(pair, 1),
            base_liquidity: liq,
            base_reserve_a: res_a,
            base_reserve_b: res_b,
            tvl_usd: 800_000.0 + f64::from(pair) * 100_000.0,
            pair_id: pair,
        });

        pools.push(PoolSpec {
            pool: pool_pubkey(pair, 1),
            dex: DexType::OrcaCLMM,
            token_a: token_mint(pair, 0),
            token_b: token_mint(pair, 1),
            base_liquidity: liq + 200_000,
            base_reserve_a: res_a,
            base_reserve_b: res_b,
            tvl_usd: 900_000.0 + f64::from(pair) * 100_000.0,
            pair_id: pair,
        });
    }

    let mut events: Vec<(u64, ReplayEvent)> = Vec::new();
    let steps = duration_secs / interval_secs.max(1);
    let t0: u64 = 1_700_000_000_000_000;

    for step in 0..steps {
        let ts = t0 + step * interval_secs * 1_000_000;
        let phase = (step % 120) as f64;

        for spec in &pools {
            let noise = ((step + u64::from(spec.pair_id) * 7) % 17) as f64 * 0.0003;
            let momentum = (phase / 120.0).sin() * 0.008;

            let mut reserve_b = spec.base_reserve_b as f64;
            reserve_b *= 1.0 + momentum + noise;

            // Cross-DEX skew: Orca pool diverges every ~90 s to create arb windows.
            // Orca pool skew: richer reserve_b → cheaper SOL on Raydium return leg.
            if spec.dex == DexType::OrcaCLMM && step % 90 < 8 {
                reserve_b *= 1.040;
            }

            let reserve_b = reserve_b.max(100.0) as u64;
            let liquidity = spec.base_liquidity + u128::from(step % 50) * 1_000;

            let update = MarketEvent::PoolUpdate(PoolUpdate {
                pool: Some(spec.pool),
                token_a_mint: Some(spec.token_a),
                token_b_mint: Some(spec.token_b),
                liquidity: Some(liquidity),
                sqrt_price: Some(1_000_000 + u128::from(reserve_b) * 10),
                fee_rate: Some(25),
            });
            events.push((ts, ReplayEvent::Market(update)));

            if step % 3 == u64::from(spec.pair_id) % 3 {
                let swap = MarketEvent::SwapEvent(SwapEvent {
                    pool: spec.pool,
                    input_mint: spec.token_a,
                    output_mint: spec.token_b,
                    amount_in: 50_000u128 + u128::from(step % 20) * 5_000,
                    amount_out: 49_500u128 + u128::from(step % 20) * 4_900,
                });
                events.push((ts + 100_000, ReplayEvent::Market(swap)));
            }
        }

        if step % 20 == 0 {
            let pool = pool_pubkey((step / 45) as u8 % 5, 0);
            let whale = WhaleEvent {
                timestamp_micros: ts + 500_000,
                pool_address: pool,
                swap_amount_usd: 25_000.0 + (step % 10) as f64 * 2_000.0,
                direction: if step % 2 == 0 {
                    Direction::Long
                } else {
                    Direction::Short
                },
                profitability_score: 0.75 + (step % 5) as f64 * 0.04,
            };
            events.push((ts + 500_000, ReplayEvent::Whale(whale)));
        }
    }

    events.sort_by_key(|(ts, _)| *ts);

    ReplayDataset {
        events,
        duration_secs,
        pool_count: pools.len(),
    }
}

/// Converts a replay event to a signal input.
pub fn to_signal_input(event: &ReplayEvent) -> Option<SignalInput> {
    match event {
        ReplayEvent::Market(me) => Some(SignalInput::Market(me.clone())),
        ReplayEvent::Whale(w) => Some(SignalInput::Whale(w.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_generates_events() {
        let ds = generate_dataset(3600, 10);
        assert!(ds.events.len() > 100);
        assert_eq!(ds.pool_count, 10);
    }
}
