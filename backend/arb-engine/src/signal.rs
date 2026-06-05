//! ArbSignal generation and TradeSignal mapping for execution-engine.

use serde::{Deserialize, Serialize};

use crate::config::ArbConfig;
use crate::spread::SpreadOpportunity;
use crate::types::ArbSignal;

/// Canonical TradeSignal for cross-process channel / WS bridge.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TradeSignalOut {
    pub v: u32,
    pub token: String,
    pub direction: String,
    pub confidence: f64,
    pub expected_edge: f64,
    pub size_usd: f64,
    pub strategy: String,
    pub timestamp_ms: u64,
}

impl TradeSignalOut {
    pub fn from_arb(arb: &ArbSignal, size_usd: f64) -> Self {
        let token = arb.token_pair.split('/').next().unwrap_or("SOL").to_string();
        Self {
            v: 1,
            token,
            direction: "long".into(),
            confidence: arb.confidence,
            expected_edge: arb.spread_pct * 100.0,
            size_usd,
            strategy: "arbitrage_capture".into(),
            timestamp_ms: crate::types::unix_ms(),
        }
    }
}

pub fn execution_size(opp: &SpreadOpportunity, config: &ArbConfig) -> f64 {
    opp.min_liquidity.min(config.max_execution_size_usd)
}

pub fn passes_routing_filters(arb: &ArbSignal, opp: &SpreadOpportunity, config: &ArbConfig) -> bool {
    if arb.confidence < config.min_confidence {
        return false;
    }
    let size = execution_size(opp, config);
    let net_spread = (arb.spread_pct - config.round_trip_fee_pct).max(0.0);
    size * net_spread >= config.min_profit_usd
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ArbSignal;

    #[test]
    fn maps_to_trade_signal() {
        let arb = ArbSignal::new("SOL/USDC", 0.005, 0.85);
        let out = TradeSignalOut::from_arb(&arb, 100.0);
        assert_eq!(out.strategy, "arbitrage_capture");
        assert_eq!(out.token, "SOL");
        assert!(out.expected_edge > 0.0);
    }
}
