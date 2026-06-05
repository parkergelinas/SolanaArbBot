//! Core types — aligned with `shared/contracts/alpha/v1.ts`.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WalletScore {
    pub wallet: String,
    pub score: f64,
    pub confidence: f64,
    pub token: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MarketSignal {
    pub token: String,
    pub momentum: f64,
    pub volume_spike: f64,
    pub price_change: f64,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MicrostructureEvent {
    pub token: String,
    pub imbalance: f64,
    pub momentum: f64,
    pub volatility: f64,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ArbSignal {
    pub token_pair: String,
    pub spread_pct: f64,
    pub confidence: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LiquiditySnapshot {
    pub token: String,
    pub liquidity_usd: f64,
    pub spread_bps: f64,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FusedFeatureVector {
    pub token: String,
    pub wallet_smart_money_score: f64,
    pub momentum_strength: f64,
    pub liquidity_conditions: f64,
    pub arbitrage_opportunity_strength: f64,
    pub volume_spike_norm: f64,
    pub price_change: f64,
    pub imbalance: f64,
    pub timestamp: u64,
    pub ttl_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScoringWeights {
    pub w1: f64,
    pub w2: f64,
    pub w3: f64,
    pub w4: f64,
    pub w5: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AlphaSignal {
    pub signal_id: String,
    pub token_in: String,
    pub token_out: String,
    pub wallet: String,
    pub confidence: f64,
    pub expected_edge: f64,
    pub size_usd: f64,
    pub strategy: String,
    pub score: f64,
    pub direction: String,
    pub timestamp: u64,
}

/// Canonical output for execution-engine.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TradeSignal {
    pub v: u32,
    pub token: String,
    pub direction: String,
    pub size_usd: f64,
    pub confidence: f64,
    pub expected_edge: f64,
    pub strategy: String,
    pub timestamp_ms: u64,
}

impl TradeSignal {
    pub fn from_alpha(alpha: &AlphaSignal) -> Self {
        Self {
            v: SCHEMA_VERSION,
            token: alpha.token_out.clone(),
            direction: alpha.direction.clone(),
            size_usd: alpha.size_usd,
            confidence: alpha.confidence,
            expected_edge: alpha.expected_edge,
            strategy: alpha.strategy.clone(),
            timestamp_ms: alpha.timestamp,
        }
    }
}

pub fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn token_from_pair(pair: &str) -> String {
    pair.split('/').next().unwrap_or(pair).to_string()
}
