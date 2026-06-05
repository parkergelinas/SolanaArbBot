//! Adapter: engine signal input → `LiveSignal`.

use crate::classify::strategy_tag_for;
use crate::types::{
    AlertType, LiveSignal, SignalKind, SignalSourceMeta, StrategyTag, SCHEMA_VERSION,
};

/// Minimal engine signal input — avoids coupling signal-bus to the `signals` crate.
#[derive(Clone, Debug)]
pub struct EngineSignalInput {
    pub signal_id: u64,
    pub timestamp_micros: u64,
    pub pool_address_hex: String,
    pub signal_type: AlertType,
    pub strength: f64,
    pub confidence: f64,
    pub direction: String,
    pub explanation: String,
}

pub fn engine_to_live_signal(input: &EngineSignalInput) -> LiveSignal {
    let strategy_tag = match input.signal_type {
        AlertType::WhaleFlow | AlertType::SmartMoney => {
            strategy_tag_for(input.signal_type, input.strength, input.confidence, None)
        }
        AlertType::Momentum => StrategyTag::Informational,
    };
    let dedup_key = format!(
        "engine:{}:{}",
        input.signal_type.as_api_str(),
        input.pool_address_hex
    );
    let id = input.signal_id.to_string();

    LiveSignal {
        v: SCHEMA_VERSION,
        signal_id: id.clone(),
        kind: SignalKind::Engine,
        source: SignalSourceMeta {
            layer: "engine".into(),
            dex: "internal".into(),
            slot: 0,
            wallet_label: None,
        },
        pair: input.pool_address_hex.clone(),
        token_in: input.pool_address_hex.clone(),
        token_out: "USDC".into(),
        timestamp_ms: input.timestamp_micros / 1_000,
        tx_id: id,
        price: 0.0,
        size: 0.0,
        confidence: input.confidence,
        wallet: String::new(),
        strength: Some(input.strength),
        size_usd: None,
        alert_type: Some(input.signal_type),
        strategy_tag: Some(strategy_tag),
        explanation: Some(input.explanation.clone()),
        direction: Some(input.direction.clone()),
        dedup_key,
    }
}
