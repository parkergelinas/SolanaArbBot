//! When `SIGNAL_HUB_URL` is set, mirror control-api's live signal buffer into stream-api.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Sender;
use signal_bus::{LiveSignal, SignalBus, SignalKind};
use tracing::{info, warn};

use crate::bridge::live_to_stream_swap;
use crate::contracts::{Signal, SignalKind as WsSignalKind, SwapEvent, WSMessage};

fn live_to_ws_signal(live: &LiveSignal) -> Option<WSMessage> {
    match live.kind {
        SignalKind::Swap => None,
        SignalKind::WhaleAlert => Some(WSMessage::Signal(Signal {
            v: 1,
            signal_id: live.signal_id.clone(),
            mint: live.token_out.clone(),
            kind: WsSignalKind::WhaleFlow,
            strength: live.strength.unwrap_or(live.confidence),
            confidence: live.confidence,
            timestamp_ms: live.timestamp_ms,
            detail: live.explanation.clone(),
        })),
        SignalKind::SmartMoneyAlert => Some(WSMessage::Signal(Signal {
            v: 1,
            signal_id: live.signal_id.clone(),
            mint: live.token_out.clone(),
            kind: WsSignalKind::SmartMoney,
            strength: live.strength.unwrap_or(live.confidence),
            confidence: live.confidence,
            timestamp_ms: live.timestamp_ms,
            detail: live.explanation.clone(),
        })),
        SignalKind::Engine => Some(WSMessage::Signal(Signal {
            v: 1,
            signal_id: live.signal_id.clone(),
            mint: live.token_out.clone(),
            kind: WsSignalKind::Momentum,
            strength: live.strength.unwrap_or(live.confidence),
            confidence: live.confidence,
            timestamp_ms: live.timestamp_ms,
            detail: live.explanation.clone(),
        })),
    }
}

/// Polls `GET {hub}/api/live-signals` and republishes into the local bus.
pub fn spawn_hub_mirror(
    hub_url: String,
    bus: Arc<SignalBus>,
    swap_tx: Sender<SwapEvent>,
    ws_tx: Sender<WSMessage>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut seen = HashSet::new();
        let poll_ms = std::env::var("SIGNAL_HUB_POLL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1_000);

        info!(%hub_url, poll_ms, "stream-api mirroring signal hub");

        loop {
            let url = format!(
                "{}/api/live-signals?limit=200",
                hub_url.trim_end_matches('/')
            );
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(signals) = resp.json::<Vec<LiveSignal>>().await {
                        for live in signals {
                            if !seen.insert(live.dedup_key.clone()) {
                                continue;
                            }
                            if let Some(accepted) = bus.publish(live.clone()).await {
                                if accepted.kind == SignalKind::Swap {
                                    let _ = swap_tx.send(live_to_stream_swap(&accepted));
                                }
                                if let Some(msg) = live_to_ws_signal(&accepted) {
                                    let _ = ws_tx.send(msg);
                                }
                            }
                        }
                    }
                }
                Ok(resp) => {
                    warn!(status = %resp.status(), "signal hub poll failed");
                }
                Err(e) => {
                    warn!(error = %e, "signal hub poll error");
                }
            }
            tokio::time::sleep(Duration::from_millis(poll_ms)).await;
        }
    });
}

/// Forward non-swap live signals from the local bus into WS batch ingress.
pub fn spawn_signal_bus_fanout(bus: Arc<SignalBus>, ws_tx: Sender<WSMessage>) {
    tokio::spawn(async move {
        for live in bus.replay().await {
            if let Some(msg) = live_to_ws_signal(&live) {
                let _ = ws_tx.send(msg);
            }
        }
        let mut sub = bus.subscribe();
        loop {
            match sub.recv().await {
                Ok(live) => {
                    if let Some(msg) = live_to_ws_signal(&live) {
                        let _ = ws_tx.send(msg);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
}
