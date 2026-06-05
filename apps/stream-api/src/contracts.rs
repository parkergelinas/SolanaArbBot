//! Wire contract v1 — must stay aligned with `shared/contracts/stream/v1.ts`.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dex {
    Raydium,
    Orca,
    Jupiter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandleInterval {
    #[serde(rename = "1s")]
    S1,
    #[serde(rename = "5s")]
    S5,
    #[serde(rename = "1m")]
    M1,
}

impl CandleInterval {
    pub fn bucket_ms(self) -> u64 {
        match self {
            Self::S1 => 1_000,
            Self::S5 => 5_000,
            Self::M1 => 60_000,
        }
    }

    pub const ALL: [Self; 3] = [Self::S1, Self::S5, Self::M1];
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwapEvent {
    pub v: u32,
    pub signature: String,
    pub dex: Dex,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: String,
    pub amount_out: String,
    pub wallet: String,
    pub slot: u64,
    pub timestamp_ms: u64,
}

impl SwapEvent {
    pub fn new_v1(
        signature: impl Into<String>,
        dex: Dex,
        token_in: impl Into<String>,
        token_out: impl Into<String>,
        amount_in: impl Into<String>,
        amount_out: impl Into<String>,
        wallet: impl Into<String>,
        slot: u64,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            v: SCHEMA_VERSION,
            signature: signature.into(),
            dex,
            token_in: token_in.into(),
            token_out: token_out.into(),
            amount_in: amount_in.into(),
            amount_out: amount_out.into(),
            wallet: wallet.into(),
            slot,
            timestamp_ms,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenPrice {
    pub v: u32,
    pub mint: String,
    pub price_usd: f64,
    pub slot: u64,
    pub timestamp_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub v: u32,
    pub mint: String,
    pub interval: CandleInterval,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub ts_open_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    Momentum,
    WhaleFlow,
    SmartMoney,
    Imbalance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    pub v: u32,
    pub signal_id: String,
    pub mint: String,
    pub kind: SignalKind,
    pub strength: f64,
    pub confidence: f64,
    pub timestamp_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum WSMessage {
    Swap(SwapEvent),
    #[serde(rename = "token_price")]
    TokenPrice(TokenPrice),
    Candle(Candle),
    Signal(Signal),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WSBatchFrame {
    pub v: u32,
    #[serde(rename = "type")]
    pub frame_type: &'static str,
    pub seq: u64,
    pub ts_ms: u64,
    pub messages: Vec<WSMessage>,
}

impl WSBatchFrame {
    pub fn new(seq: u64, messages: Vec<WSMessage>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_message_roundtrip() {
        let swap = SwapEvent::new_v1(
            "sig",
            Dex::Raydium,
            "SOL",
            "USDC",
            "1000",
            "150000",
            "wallet",
            1,
            1_700_000_000_000,
        );
        let msg = WSMessage::Swap(swap);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"swap\""));
        let back: WSMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, WSMessage::Swap(_)));
    }

    #[test]
    fn batch_frame_has_version_and_type() {
        let frame = WSBatchFrame::new(0, vec![]);
        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"type\":\"batch\""));
    }
}
