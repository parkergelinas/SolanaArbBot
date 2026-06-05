//! Intelligence whale / smart-money wire payloads → `LiveSignal`.

use serde::Deserialize;

use crate::adapter::SOL_MINT;
use crate::classify::strategy_tag_for;
use crate::types::{
    AlertType, LiveSignal, SignalKind, SignalSourceMeta, StrategyTag, SCHEMA_VERSION,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RawWhaleAlert {
    pub alert_id: String,
    pub signature: String,
    pub wallet: String,
    pub token: String,
    pub token_symbol: String,
    pub dex: String,
    pub amount_sol: f64,
    pub notional_usd: f64,
    pub strength: f64,
    pub confidence: f64,
    pub timestamp: u64,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawSmartMoneyAlert {
    pub alert_id: String,
    pub signature: String,
    pub wallet: String,
    pub token: String,
    pub token_symbol: String,
    pub dex: String,
    pub amount_sol: f64,
    pub notional_usd: f64,
    pub strength: f64,
    pub confidence: f64,
    pub timestamp: u64,
    pub detail: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawIntelligenceBatch {
    #[serde(rename = "type")]
    pub frame_type: String,
    pub messages: Vec<RawIntelligenceMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum RawIntelligenceMessage {
    WhaleAlert(RawWhaleAlert),
    #[serde(rename = "smart_money_alert")]
    SmartMoneyAlert(RawSmartMoneyAlert),
}

pub fn alert_id_hash(alert_id: &str) -> u64 {
    crate::types::alert_id_hash(alert_id)
}

pub fn whale_to_live_signal(alert: &RawWhaleAlert) -> LiveSignal {
    let alert_type = AlertType::WhaleFlow;
    let strategy_tag =
        strategy_tag_for(alert_type, alert.strength, alert.confidence, Some(alert.notional_usd));
    build_whale_signal(alert, strategy_tag)
}

pub fn smart_money_to_live_signal(alert: &RawSmartMoneyAlert) -> LiveSignal {
    let alert_type = AlertType::SmartMoney;
    let strategy_tag =
        strategy_tag_for(alert_type, alert.strength, alert.confidence, Some(alert.notional_usd));
    build_smart_money_signal(alert, strategy_tag)
}

fn build_whale_signal(alert: &RawWhaleAlert, strategy_tag: StrategyTag) -> LiveSignal {
    let explanation = alert.detail.clone().unwrap_or_else(|| {
        format!(
            "Whale {:.2} SOL (~${:.0}) on {} · {}",
            alert.amount_sol, alert.notional_usd, alert.dex, alert.token_symbol
        )
    });
    LiveSignal {
        v: SCHEMA_VERSION,
        signal_id: alert.alert_id.clone(),
        kind: SignalKind::WhaleAlert,
        source: SignalSourceMeta {
            layer: "intelligence".into(),
            dex: alert.dex.clone(),
            slot: 0,
            wallet_label: Some("whale".into()),
        },
        pair: format!("SOL/{}", alert.token_symbol),
        token_in: SOL_MINT.into(),
        token_out: alert.token.clone(),
        timestamp_ms: alert.timestamp,
        tx_id: alert.signature.clone(),
        price: if alert.amount_sol > 0.0 {
            alert.notional_usd / alert.amount_sol
        } else {
            0.0
        },
        size: alert.amount_sol,
        confidence: alert.confidence,
        wallet: alert.wallet.clone(),
        strength: Some(alert.strength),
        size_usd: Some(alert.notional_usd),
        alert_type: Some(AlertType::WhaleFlow),
        strategy_tag: Some(strategy_tag),
        explanation: Some(explanation),
        direction: Some("Long".into()),
        dedup_key: format!("whale:{}:{}", alert.wallet, alert.token),
    }
}

fn build_smart_money_signal(alert: &RawSmartMoneyAlert, strategy_tag: StrategyTag) -> LiveSignal {
    let explanation = alert.detail.clone().unwrap_or_else(|| {
        format!(
            "Smart money {:.2} SOL on {} · {}",
            alert.amount_sol, alert.dex, alert.token_symbol
        )
    });
    LiveSignal {
        v: SCHEMA_VERSION,
        signal_id: alert.alert_id.clone(),
        kind: SignalKind::SmartMoneyAlert,
        source: SignalSourceMeta {
            layer: "intelligence".into(),
            dex: alert.dex.clone(),
            slot: 0,
            wallet_label: Some("smart".into()),
        },
        pair: format!("SOL/{}", alert.token_symbol),
        token_in: SOL_MINT.into(),
        token_out: alert.token.clone(),
        timestamp_ms: alert.timestamp,
        tx_id: alert.signature.clone(),
        price: if alert.amount_sol > 0.0 {
            alert.notional_usd / alert.amount_sol
        } else {
            0.0
        },
        size: alert.amount_sol,
        confidence: alert.confidence,
        wallet: alert.wallet.clone(),
        strength: Some(alert.strength),
        size_usd: Some(alert.notional_usd),
        alert_type: Some(AlertType::SmartMoney),
        strategy_tag: Some(strategy_tag),
        explanation: Some(explanation),
        direction: Some("Long".into()),
        dedup_key: format!("smart:{}:{}", alert.wallet, alert.token),
    }
}

/// Parse intelligence WS batch; returns whale + smart-money signals only.
pub fn parse_batch_frame(text: &str) -> Result<Vec<LiveSignal>, serde_json::Error> {
    let batch: RawIntelligenceBatch = serde_json::from_str(text)?;
    if batch.frame_type != "batch" {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for msg in batch.messages {
        match msg {
            RawIntelligenceMessage::WhaleAlert(a) => out.push(whale_to_live_signal(&a)),
            RawIntelligenceMessage::SmartMoneyAlert(a) => out.push(smart_money_to_live_signal(&a)),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whale_adapter_maps_fields() {
        let alert = RawWhaleAlert {
            alert_id: "whale_1".to_owned(),
            signature: "sig".to_owned(),
            wallet: "wallet_a".to_owned(),
            token: "token_mint".to_owned(),
            token_symbol: "BONK".to_owned(),
            dex: "raydium".to_owned(),
            amount_sol: 50.0,
            notional_usd: 10_000.0,
            strength: 0.9,
            confidence: 0.85,
            timestamp: 1_700_000_000,
            detail: None,
        };
        let live = whale_to_live_signal(&alert);
        assert_eq!(live.alert_type, Some(AlertType::WhaleFlow));
        assert_eq!(live.kind, SignalKind::WhaleAlert);
        assert_eq!(live.strategy_tag, Some(StrategyTag::WhaleCopyCandidate));
    }
}
