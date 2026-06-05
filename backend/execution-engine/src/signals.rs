//! Canonical trade signal — aligned with `shared/contracts/execution/v1.ts`.

use serde::{Deserialize, Serialize};

use crate::config::EngineConfig;

pub const SCHEMA_VERSION: u32 = 1;

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
    pub fn new(
        token: impl Into<String>,
        direction: impl Into<String>,
        confidence: f64,
        expected_edge: f64,
        size_usd: f64,
        strategy: impl Into<String>,
    ) -> Self {
        Self {
            v: SCHEMA_VERSION,
            token: token.into(),
            direction: direction.into(),
            size_usd,
            confidence,
            expected_edge,
            strategy: strategy.into(),
            timestamp_ms: unix_ms(),
        }
    }

    pub fn is_long(&self) -> bool {
        self.direction.eq_ignore_ascii_case("long")
    }

    /// Resolve Jupiter swap mints from direction + target token.
    pub fn resolve_mints(&self, config: &EngineConfig) -> (String, String) {
        if self.is_long() {
            (config.quote_mint.clone(), self.token.clone())
        } else {
            (self.token.clone(), config.quote_mint.clone())
        }
    }
}

pub fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
