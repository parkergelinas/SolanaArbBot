//! Async signal feed — demo, arb WS, alpha-engine bridge.

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::signals::TradeSignal;

const SOL: &str = "So11111111111111111111111111111111111111112";

#[derive(serde::Deserialize)]
struct ArbSignalWire {
    token_pair: String,
    spread_pct: f64,
    confidence: f64,
}

pub fn spawn_intelligence_stub(_tx: mpsc::UnboundedSender<TradeSignal>) {
    info!("intelligence signal bridge stub — wire to data-layer whale alerts");
}

#[derive(serde::Deserialize)]
struct HubLiveSignal {
    signal_id: String,
    kind: String,
    #[serde(default)]
    strategy_tag: Option<String>,
    #[serde(default)]
    token_in: String,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    size: f64,
    #[serde(default)]
    size_usd: Option<f64>,
    #[serde(default)]
    strength: Option<f64>,
}

/// Poll `SIGNAL_HUB_URL` / `CONTROL_API_URL` `/api/live-signals` for whale-copy candidates.
pub fn spawn_signal_hub_subscriber(tx: mpsc::UnboundedSender<TradeSignal>) {
    let hub = std::env::var("SIGNAL_HUB_URL")
        .or_else(|_| std::env::var("CONTROL_API_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:3001".into());
    let hub = hub.trim_end_matches('/').to_string();
    let poll_ms: u64 = std::env::var("SIGNAL_HUB_POLL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000);

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{hub}/api/live-signals?limit=50");
        let mut seen = std::collections::HashSet::<String>::new();
        info!(url = %url, "signal-hub → execution-engine subscriber started");

        loop {
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(signals) = resp.json::<Vec<HubLiveSignal>>().await {
                        for s in signals {
                            if !seen.insert(s.signal_id.clone()) {
                                continue;
                            }
                            if let Some(sig) = hub_to_trade(&s) {
                                if tx.send(sig).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
                Ok(resp) => warn!(status = %resp.status(), "signal-hub poll failed"),
                Err(e) => warn!(error = %e, "signal-hub poll error"),
            }
            tokio::time::sleep(std::time::Duration::from_millis(poll_ms)).await;
        }
    });
}

fn hub_to_trade(s: &HubLiveSignal) -> Option<TradeSignal> {
    let tag = s.strategy_tag.as_deref().unwrap_or("");
    let is_whale = s.kind.contains("whale") || tag == "whale_copy_candidate";
    if !is_whale && s.kind != "smart_money_alert" {
        return None;
    }

    let strategy = if is_whale {
        "whale_copy_trade"
    } else {
        "momentum_follow"
    };

    let edge = s.strength.unwrap_or(s.confidence) * 100.0;
    let size = s.size_usd.unwrap_or(s.size * 150.0);

    Some(TradeSignal::new(
        if s.token_in.is_empty() {
            "So11111111111111111111111111111111111111112".into()
        } else {
            s.token_in.clone()
        },
        "long",
        s.confidence,
        edge,
        size.max(10.0),
        strategy,
    ))
}

pub fn spawn_demo_signal_feed(tx: mpsc::UnboundedSender<TradeSignal>, interval_ms: u64) {
    tokio::spawn(async move {
        let mut n: u64 = 0;
        let strategies = ["momentum_follow", "whale_copy_trade", "sniper_entry"];
        loop {
            n += 1;
            let strategy = strategies[(n as usize) % strategies.len()];
            let signal = TradeSignal::new(
                SOL,
                "long",
                0.75 + (n % 3) as f64 * 0.05,
                12.0 + (n % 5) as f64,
                25.0,
                strategy,
            );
            if tx.send(signal).is_err() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
        }
    });
    info!("demo signal feed started (interval {interval_ms}ms)");
}

pub fn spawn_arb_ws_subscriber(ws_url: String, tx: mpsc::UnboundedSender<TradeSignal>) {
    info!(url = %ws_url, "arb-engine WS subscriber started");
    tokio::spawn(async move {
        loop {
            match connect_arb_ws(&ws_url, tx.clone()).await {
                Ok(()) => warn!("arb WS closed, reconnecting in 2s"),
                Err(e) => warn!(error = %e, "arb WS error, reconnecting in 2s"),
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}

pub fn spawn_alpha_channel_bridge(
    mut rx: mpsc::UnboundedReceiver<TradeSignal>,
    tx: mpsc::UnboundedSender<TradeSignal>,
) {
    tokio::spawn(async move {
        while let Some(signal) = rx.recv().await {
            if tx.send(signal).is_err() {
                break;
            }
        }
    });
    info!("alpha-engine → execution-engine channel bridge active");
}

async fn connect_arb_ws(
    ws_url: &str,
    tx: mpsc::UnboundedSender<TradeSignal>,
) -> anyhow::Result<()> {
    use futures_util::StreamExt;
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let (ws, _) = connect_async(ws_url).await?;
    let (_, mut read) = ws.split();

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            if let Ok(arb) = serde_json::from_str::<ArbSignalWire>(&text) {
                if let Some(signal) = arb_to_trade_signal(&arb) {
                    if tx.send(signal).is_err() {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

fn arb_to_trade_signal(arb: &ArbSignalWire) -> Option<TradeSignal> {
    let token = arb.token_pair.split('/').next()?.to_string();
    Some(TradeSignal::new(
        token,
        "long",
        arb.confidence,
        arb.spread_pct * 100.0,
        100.0,
        "arbitrage_capture",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_arb_wire_to_trade_signal() {
        let arb = ArbSignalWire {
            token_pair: "SOL/USDC".into(),
            spread_pct: 0.005,
            confidence: 0.85,
        };
        let sig = arb_to_trade_signal(&arb).expect("mapped");
        assert_eq!(sig.strategy, "arbitrage_capture");
        assert_eq!(sig.token, "SOL");
    }
}
