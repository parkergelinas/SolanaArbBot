//! Core price and arbitrage signal types — aligned with `shared/contracts/arb/v1.ts`.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

/// Normalized pool price from a single DEX venue.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PoolPrice {
    pub dex: String,
    pub token_a: String,
    pub token_b: String,
    pub price: f64,
    pub liquidity: f64,
    pub timestamp: u64,
}

impl PoolPrice {
    pub fn pair_key(&self) -> String {
        canonical_pair(&self.token_a, &self.token_b)
    }

    pub fn pool_key(&self) -> (String, String) {
        (self.pair_key(), self.dex.clone())
    }
}

/// Wire-format cross-DEX arbitrage signal (minimal).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ArbSignal {
    pub v: u32,
    pub token_pair: String,
    pub spread_pct: f64,
    pub confidence: f64,
}

impl ArbSignal {
    pub fn new(token_pair: impl Into<String>, spread_pct: f64, confidence: f64) -> Self {
        Self {
            v: SCHEMA_VERSION,
            token_pair: token_pair.into(),
            spread_pct,
            confidence,
        }
    }
}

pub fn canonical_pair(a: &str, b: &str) -> String {
    if a <= b {
        format!("{a}/{b}")
    } else {
        format!("{b}/{a}")
    }
}

pub fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
