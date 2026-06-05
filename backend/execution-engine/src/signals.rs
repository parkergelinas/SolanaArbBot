//! Input trade signal model — aligned with `shared/contracts/execution/v1.ts`.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TradeSignal {
    pub v: u32,
    pub wallet: String,
    pub token_in: String,
    pub token_out: String,
    pub confidence: f64,
    pub expected_edge: f64,
    pub size_usd: f64,
    pub strategy: String,
    pub timestamp_ms: u64,
}

impl TradeSignal {
    pub fn new(
        wallet: impl Into<String>,
        token_in: impl Into<String>,
        token_out: impl Into<String>,
        confidence: f64,
        expected_edge: f64,
        size_usd: f64,
        strategy: impl Into<String>,
    ) -> Self {
        Self {
            v: SCHEMA_VERSION,
            wallet: wallet.into(),
            token_in: token_in.into(),
            token_out: token_out.into(),
            confidence,
            expected_edge,
            size_usd,
            strategy: strategy.into(),
            timestamp_ms: unix_ms(),
        }
    }
}

pub fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
