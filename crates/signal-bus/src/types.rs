//! Normalized live signal — swaps, whale alerts, and engine signals share one schema.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignalSourceMeta {
    /// Origin layer: `data-layer`, `intelligence`, `engine`.
    pub layer: String,
    pub dex: String,
    pub slot: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_label: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    Swap,
    WhaleAlert,
    SmartMoneyAlert,
    Engine,
}

/// Alert taxonomy aligned with `signals::SignalType` and intelligence contracts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    WhaleFlow,
    SmartMoney,
    Momentum,
}

impl AlertType {
    pub fn as_api_str(self) -> &'static str {
        match self {
            Self::WhaleFlow => "WhaleFlow",
            Self::SmartMoney => "SmartMoney",
            Self::Momentum => "Momentum",
        }
    }
}

/// Strategy classification for downstream bot routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyTag {
    WhaleCopyCandidate,
    WatchOnly,
    Informational,
}

/// Canonical normalized signal stored in the shared buffer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveSignal {
    pub v: u32,
    /// Transaction signature or alert id.
    pub signal_id: String,
    pub kind: SignalKind,
    pub source: SignalSourceMeta,
    pub pair: String,
    pub token_in: String,
    pub token_out: String,
    pub timestamp_ms: u64,
    pub tx_id: String,
    pub price: f64,
    /// Size in SOL (or native units for swaps).
    pub size: f64,
    pub confidence: f64,
    pub wallet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_type: Option<AlertType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_tag: Option<StrategyTag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip)]
    pub dedup_key: String,
}

impl LiveSignal {
    pub fn timestamp_micros(&self) -> u64 {
        self.timestamp_ms.saturating_mul(1_000)
    }

    pub fn numeric_id(&self) -> u64 {
        alert_id_hash(&self.signal_id)
    }
}

/// Stable id hash from alert / tx string (matches dashboard `alertId()`).
pub fn alert_id_hash(id: &str) -> u64 {
    let mut h: i32 = 0;
    for b in id.bytes() {
        h = h.wrapping_mul(31).wrapping_add(i32::from(b));
    }
    let u = u64::from(h as u32);
    if h > 0 { u64::MAX - u + 1 } else { u }
}
