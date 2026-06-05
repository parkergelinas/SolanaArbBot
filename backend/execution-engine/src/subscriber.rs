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
