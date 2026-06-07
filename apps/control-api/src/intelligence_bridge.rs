//! Intelligence-api whale alerts → shared signal-bus → control-api WS/REST.

use std::sync::Arc;

use crossbeam_channel::Receiver;
use futures_util::StreamExt;
use intelligence_api::contracts::{IntelligenceMessage, SmartMoneyAlert, WhaleAlert};
use intelligence_api::pipeline::spawn_intelligence_pipeline;
use signal_bus::{
    adapter::{smart_money_to_live_signal, whale_to_live_signal, RawSmartMoneyAlert, RawWhaleAlert},
    parse_batch_frame, SignalBus,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

use crate::dto::SignalEventDto;
use crate::state::AppState;

fn whale_contract_to_live(alert: &WhaleAlert) -> signal_bus::LiveSignal {
    let raw = RawWhaleAlert {
        alert_id: alert.alert_id.clone(),
        signature: alert.signature.clone(),
        wallet: alert.wallet.clone(),
        token: alert.token.clone(),
        token_symbol: alert.token_symbol.clone(),
        dex: format!("{:?}", alert.dex).to_lowercase(),
        amount_sol: alert.amount_sol,
        notional_usd: alert.notional_usd,
        strength: alert.strength,
        confidence: alert.confidence,
        timestamp: alert.timestamp,
        detail: alert.detail.clone(),
    };
    whale_to_live_signal(&raw)
}

fn smart_contract_to_live(alert: &SmartMoneyAlert) -> signal_bus::LiveSignal {
    let raw = RawSmartMoneyAlert {
        alert_id: alert.alert_id.clone(),
        signature: alert.signature.clone(),
        wallet: alert.wallet.clone(),
        token: alert.token.clone(),
        token_symbol: alert.token_symbol.clone(),
        dex: format!("{:?}", alert.dex).to_lowercase(),
        amount_sol: alert.amount_sol,
        notional_usd: alert.notional_usd,
        strength: alert.strength,
        confidence: alert.confidence,
        timestamp: alert.timestamp,
        detail: alert.detail.clone(),
    };
    smart_money_to_live_signal(&raw)
}

fn dispatch_intelligence_message(
    bus: &SignalBus,
    state: &AppState,
    msg: IntelligenceMessage,
    rt: &tokio::runtime::Handle,
) {
    let live = match msg {
        IntelligenceMessage::WhaleAlert(a) => whale_contract_to_live(&a),
        IntelligenceMessage::SmartMoneyAlert(a) => smart_contract_to_live(&a),
        _ => return,
    };
    if let Some(accepted) = bus.publish_blocking(live) {
        let dto = SignalEventDto::from(&accepted);
        let state = state.clone();
        // Use the captured tokio Handle — `tokio::spawn` would panic here because
        // this runs on a bare `std::thread`, outside any async context.
        rt.spawn(async move {
            state.ingest_signal(dto).await;
        });
    }
}

/// In-process: consume data-layer `whale_rx` fan-out through intelligence pipeline.
pub fn spawn_in_process_intelligence_bridge(
    pipeline_rx: Receiver<data_layer::types::PipelineEvent>,
    whale_threshold_sol: f64,
    bus: Arc<SignalBus>,
    state: AppState,
) {
    let (msg_tx, msg_rx) = crossbeam_channel::unbounded();
    spawn_intelligence_pipeline(pipeline_rx, msg_tx, whale_threshold_sol);

    // Capture the tokio Handle from the current async context BEFORE spawning
    // the OS thread — the thread itself has no runtime and cannot call
    // `tokio::spawn` or `Handle::current()` directly.
    let rt = tokio::runtime::Handle::current();

    std::thread::spawn(move || {
        info!("intelligence in-process bridge online (whale threshold {whale_threshold_sol} SOL)");
        while let Ok(msg) = msg_rx.recv() {
            dispatch_intelligence_message(&bus, &state, msg, &rt);
        }
    });
}

/// WS subscriber to a running intelligence-api (`ws://host:8090/intelligence`).
pub fn spawn_ws_intelligence_bridge(url: String, bus: Arc<SignalBus>, state: AppState) {
    tokio::spawn(async move {
        let mut backoff_ms = 1_000u64;
        loop {
            info!(%url, "connecting intelligence WS bridge");
            match connect_async(&url).await {
                Ok((ws, _)) => {
                    backoff_ms = 1_000;
                    let (_, mut read) = ws.split();
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                match parse_batch_frame(&text) {
                                    Ok(signals) => {
                                        for live in signals {
                                            if let Some(accepted) = bus.publish(live).await {
                                                let dto = SignalEventDto::from(&accepted);
                                                state.ingest_signal(dto).await;
                                            }
                                        }
                                    }
                                    Err(e) => warn!("intelligence batch parse error: {e}"),
                                }
                            }
                            Ok(Message::Close(_)) | Err(_) => break,
                            _ => {}
                        }
                    }
                }
                Err(e) => warn!("intelligence WS connect failed: {e}"),
            }
            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
            backoff_ms = (backoff_ms * 2).min(30_000);
        }
    });
}

/// Start bridge based on `INTELLIGENCE_BRIDGE` env.
pub fn spawn_intelligence_bridge(
    whale_rx: crossbeam_channel::Receiver<data_layer::types::PipelineEvent>,
    whale_threshold_sol: f64,
    bus: Arc<SignalBus>,
    state: AppState,
) {
    let mode = std::env::var("INTELLIGENCE_BRIDGE")
        .unwrap_or_else(|_| "in_process".into())
        .to_lowercase();

    match mode.as_str() {
        "off" | "false" | "0" => {
            info!("intelligence bridge disabled (INTELLIGENCE_BRIDGE=off)");
        }
        "ws" | "websocket" => {
            let url = std::env::var("INTELLIGENCE_WS_URL")
                .unwrap_or_else(|_| "ws://127.0.0.1:8090/intelligence".into());
            spawn_ws_intelligence_bridge(url, bus, state);
        }
        _ => {
            spawn_in_process_intelligence_bridge(whale_rx, whale_threshold_sol, bus, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_env::DeployEnv;
    use config::SystemConfig;
    use signal_bus::adapter::whale_to_live_signal;
    use signal_bus::RawWhaleAlert;

    #[tokio::test]
    async fn whale_alert_reaches_signal_store_via_bus() {
        std::env::set_var("SIGNAL_BUFFER_PERSIST", "false");
        let bus = Arc::new(SignalBus::with_defaults());
        let state = AppState::new(SystemConfig::default(), DeployEnv::Development);

        let raw = RawWhaleAlert {
            alert_id: "whale_test_1".into(),
            signature: "sig".into(),
            wallet: "whale_wallet".into(),
            token: "mint".into(),
            token_symbol: "BONK".into(),
            dex: "raydium".into(),
            amount_sol: 100.0,
            notional_usd: 20_000.0,
            strength: 0.95,
            confidence: 0.9,
            timestamp: 1_700_000_000,
            detail: None,
        };
        let live = whale_to_live_signal(&raw);
        let accepted = bus.publish(live).await.expect("accepted");
        state.ingest_signal(SignalEventDto::from(&accepted)).await;

        let store = state.signal_store.lock().await;
        assert_eq!(store.len(), 1);
        assert_eq!(store[0].signal_type, "WhaleFlow");
        assert_eq!(store[0].source.as_deref(), Some("intelligence"));
        assert_eq!(
            store[0].strategy_tag.as_deref(),
            Some("whale_copy_candidate")
        );
        std::env::remove_var("SIGNAL_BUFFER_PERSIST");
    }
}
