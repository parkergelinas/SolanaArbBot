//! Spread calculator — cross-DEX price divergence per token pair.

use crate::config::ArbConfig;
use crate::pool_state::{PoolEntry, PoolStateEngine};
use crate::types::{canonical_pair, unix_ms, ArbSignal};

#[derive(Clone, Debug, PartialEq)]
pub struct SpreadOpportunity {
    pub token_pair: String,
    pub buy_dex: String,
    pub sell_dex: String,
    pub buy_price: f64,
    pub sell_price: f64,
    pub spread_pct: f64,
    pub min_liquidity: f64,
    pub avg_volatility: f64,
}

pub fn is_stale(timestamp: u64, now: u64, max_stale_ms: u64) -> bool {
    now.saturating_sub(timestamp) > max_stale_ms
}

pub fn compute_spreads(engine: &PoolStateEngine, config: &ArbConfig) -> Vec<SpreadOpportunity> {
    let now = unix_ms();
    let mut opportunities = Vec::new();

    for pair in engine.pairs() {
        let venues = engine.venues_for_pair(&pair);
        if venues.len() < 2 {
            continue;
        }

        let fresh: Vec<(String, PoolEntry)> = venues
            .into_iter()
            .filter(|(_, e)| !is_stale(e.timestamp, now, config.max_stale_ms))
            .filter(|(_, e)| e.liquidity >= config.min_liquidity_usd)
            .collect();

        if fresh.len() < 2 {
            continue;
        }

        let mut min_entry: Option<(String, f64, f64)> = None;
        let mut max_entry: Option<(String, f64, f64)> = None;

        for (dex, entry) in &fresh {
            match &min_entry {
                Some((_, p, _)) if entry.price >= *p => {}
                _ => min_entry = Some((dex.clone(), entry.price, entry.liquidity)),
            }
            match &max_entry {
                Some((_, p, _)) if entry.price <= *p => {}
                _ => max_entry = Some((dex.clone(), entry.price, entry.liquidity)),
            }
        }

        let (Some((buy_dex, buy_price, buy_liq)), Some((sell_dex, sell_price, sell_liq))) =
            (min_entry, max_entry)
        else {
            continue;
        };

        if buy_dex == sell_dex || buy_price <= 0.0 {
            continue;
        }

        let spread = sell_price - buy_price;
        let spread_pct = spread / buy_price;

        if spread_pct <= config.min_spread_pct {
            continue;
        }

        let min_liquidity = buy_liq.min(sell_liq);
        let avg_volatility = fresh.iter().map(|(_, e)| e.volatility_ema).sum::<f64>()
            / fresh.len() as f64;

        opportunities.push(SpreadOpportunity {
            token_pair: pair,
            buy_dex,
            sell_dex,
            buy_price,
            sell_price,
            spread_pct,
            min_liquidity,
            avg_volatility,
        });
    }

    opportunities
}

pub fn opportunity_to_signal(opp: &SpreadOpportunity, config: &ArbConfig) -> ArbSignal {
    let size_usd = opp.min_liquidity.min(config.max_execution_size_usd);
    let net_spread = (opp.spread_pct - config.round_trip_fee_pct).max(0.0);
    let expected_profit_usd = size_usd * net_spread;

    let liquidity_score = (opp.min_liquidity / (opp.min_liquidity + 50_000.0)).clamp(0.0, 1.0);
    let spread_score = (opp.spread_pct / (opp.spread_pct + 0.005)).clamp(0.0, 1.0);
    let vol_score = 1.0 / (1.0 + opp.avg_volatility * 20.0);
    let confidence = (liquidity_score * 0.4 + spread_score * 0.4 + vol_score * 0.2).clamp(0.0, 1.0);

    let _ = expected_profit_usd;
    ArbSignal::new(&opp.token_pair, opp.spread_pct, confidence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PoolPrice;

    fn cfg() -> ArbConfig {
        ArbConfig {
            min_spread_pct: 0.003,
            min_liquidity_usd: 10_000.0,
            max_stale_ms: 3000,
            min_profit_usd: 1.0,
            min_confidence: 0.7,
            round_trip_fee_pct: 0.003,
            batch_interval_ms: 40,
            mock_interval_ms: 50,
            ws_port: 8091,
            ws_enabled: false,
            max_execution_size_usd: 500.0,
        }
    }

    fn seed(engine: &PoolStateEngine, dex: &str, price: f64, liq: f64, ts: u64) {
        engine.upsert(&PoolPrice {
            dex: dex.into(),
            token_a: "SOL".into(),
            token_b: "USDC".into(),
            price,
            liquidity: liq,
            timestamp: ts,
        });
    }

    #[test]
    fn spread_calc_detects_cross_dex_gap() {
        let engine = PoolStateEngine::new();
        let now = unix_ms();
        seed(&engine, "raydium", 145.0, 50_000.0, now);
        seed(&engine, "orca", 146.0, 60_000.0, now);

        let opps = compute_spreads(&engine, &cfg());
        assert_eq!(opps.len(), 1);
        assert!((opps[0].spread_pct - 1.0 / 145.0).abs() < 0.0001);
        assert_eq!(opps[0].buy_dex, "raydium");
        assert_eq!(opps[0].sell_dex, "orca");
    }

    #[test]
    fn rejects_stale_prices() {
        let engine = PoolStateEngine::new();
        let now = unix_ms();
        seed(&engine, "raydium", 145.0, 50_000.0, now - 10_000);
        seed(&engine, "orca", 146.0, 60_000.0, now);

        let opps = compute_spreads(&engine, &cfg());
        assert!(opps.is_empty());
    }

    #[test]
    fn rejects_thin_liquidity() {
        let engine = PoolStateEngine::new();
        let now = unix_ms();
        seed(&engine, "raydium", 145.0, 1_000.0, now);
        seed(&engine, "orca", 146.0, 60_000.0, now);

        let opps = compute_spreads(&engine, &cfg());
        assert!(opps.is_empty());
    }

    #[test]
    fn canonical_pair_ordering() {
        assert_eq!(canonical_pair("USDC", "SOL"), "SOL/USDC");
    }
}
