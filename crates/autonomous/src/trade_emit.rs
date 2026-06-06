//! Trade lifecycle payloads emitted by the autonomous runtime.

use std::sync::atomic::{AtomicU64, Ordering};

static TRADE_SEQ: AtomicU64 = AtomicU64::new(1);

/// Generate a unique trade id for paper execution.
pub fn new_trade_id(prefix: &str) -> String {
    let n = TRADE_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{n}")
}

/// Lifecycle stage — mirrors `shared/contracts/trade/v1.ts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeStage {
    Started,
    Quoted,
    Validated,
    Submitted,
    Filled,
    Failed,
    Rejected,
    Canceled,
}

impl TradeStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Quoted => "quoted",
            Self::Validated => "validated",
            Self::Submitted => "submitted",
            Self::Filled => "filled",
            Self::Failed => "failed",
            Self::Rejected => "rejected",
            Self::Canceled => "canceled",
        }
    }
}

/// Plain trade event emitted to control-api via callback (no control-api dependency).
#[derive(Clone, Debug)]
pub struct TradeEmit {
    pub trade_id: String,
    pub wallet_id: String,
    pub source_strategy: String,
    pub pair: String,
    pub side: String,
    pub size_usd: f64,
    pub expected_pnl_usd: f64,
    pub tx_signature: Option<String>,
    pub timestamp_us: u64,
    pub stage: TradeStage,
    pub mode: String,
    pub reject_reason: Option<String>,
    pub signal_id: Option<u64>,
}

impl TradeEmit {
    pub fn paper(
        trade_id: &str,
        strategy: &str,
        pair: &str,
        side: &str,
        size_usd: f64,
        expected_pnl_usd: f64,
        timestamp_us: u64,
        stage: TradeStage,
        signal_id: Option<u64>,
    ) -> Self {
        Self {
            trade_id: trade_id.to_owned(),
            wallet_id: "paper".to_owned(),
            source_strategy: strategy.to_owned(),
            pair: pair.to_owned(),
            side: side.to_owned(),
            size_usd,
            expected_pnl_usd,
            tx_signature: None,
            timestamp_us,
            stage,
            mode: "paper".to_owned(),
            reject_reason: None,
            signal_id,
        }
    }

    /// Live trade lifecycle event tied to a real wallet pubkey.
    pub fn live(
        trade_id: &str,
        wallet_id: &str,
        strategy: &str,
        pair: &str,
        side: &str,
        size_usd: f64,
        expected_pnl_usd: f64,
        timestamp_us: u64,
        stage: TradeStage,
        signal_id: Option<u64>,
    ) -> Self {
        Self {
            trade_id: trade_id.to_owned(),
            wallet_id: wallet_id.to_owned(),
            source_strategy: strategy.to_owned(),
            pair: pair.to_owned(),
            side: side.to_owned(),
            size_usd,
            expected_pnl_usd,
            tx_signature: None,
            timestamp_us,
            stage,
            mode: "live".to_owned(),
            reject_reason: None,
            signal_id,
        }
    }

    pub fn with_tx(mut self, sig: impl Into<String>) -> Self {
        self.tx_signature = Some(sig.into());
        self
    }

    pub fn with_reject(mut self, reason: impl Into<String>) -> Self {
        self.reject_reason = Some(reason.into());
        self
    }
}
