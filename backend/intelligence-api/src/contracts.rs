//! Wire contract v1 — aligned with `shared/contracts/intelligence/v1.ts`.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dex {
    Raydium,
    Orca,
    Jupiter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowSide {
    Buy,
    Sell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletTier {
    Whale,
    Smart,
    Active,
    Retail,
}

/// Canonical normalized swap (from data-layer).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SwapEvent {
    pub signature: String,
    pub wallet: String,
    pub token: String,
    pub amount_sol: f64,
    pub dex: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnrichedSwapEvent {
    pub v: u32,
    #[serde(flatten)]
    pub swap: SwapEvent,
    pub token_symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_label: Option<String>,
    pub notional_usd: f64,
    pub slot: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawUpdate {
    pub slot: u64,
    pub signature: String,
    pub logs: Vec<String>,
    pub accounts: Vec<String>,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhaleAlert {
    pub v: u32,
    pub alert_id: String,
    pub signature: String,
    pub wallet: String,
    pub token: String,
    pub token_symbol: String,
    pub dex: Dex,
    pub amount_sol: f64,
    pub notional_usd: f64,
    pub side: FlowSide,
    pub strength: f64,
    pub confidence: f64,
    pub tier: WalletTier,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmartMoneyAlert {
    pub v: u32,
    pub alert_id: String,
    pub signature: String,
    pub wallet: String,
    pub token: String,
    pub token_symbol: String,
    pub dex: Dex,
    pub amount_sol: f64,
    pub notional_usd: f64,
    pub strength: f64,
    pub confidence: f64,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub v: u32,
    pub wallet: String,
    pub tier: WalletTier,
    pub swap_count: u32,
    pub volume_sol_24h: f64,
    pub net_flow_sol: f64,
    pub win_proxy: f64,
    pub last_token: String,
    pub last_amount_sol: f64,
    pub last_dex: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum IntelligenceMessage {
    Swap(SwapEvent),
    #[serde(rename = "enriched_swap")]
    EnrichedSwap(EnrichedSwapEvent),
    WhaleAlert(WhaleAlert),
    #[serde(rename = "smart_money_alert")]
    SmartMoneyAlert(SmartMoneyAlert),
    WalletSnapshot(WalletSnapshot),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntelligenceBatch {
    pub v: u32,
    #[serde(rename = "type")]
    pub frame_type: &'static str,
    pub seq: u64,
    pub ts_ms: u64,
    pub messages: Vec<IntelligenceMessage>,
}

impl IntelligenceBatch {
    pub fn new(seq: u64, messages: Vec<IntelligenceMessage>) -> Self {
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            v: SCHEMA_VERSION,
            frame_type: "batch",
            seq,
            ts_ms,
            messages,
        }
    }
}
