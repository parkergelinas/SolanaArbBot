//! Bridges `signal-bus` live signals into the autonomous external ingestion buffer.

use std::sync::Arc;

use autonomous::ExternalIngestionBuffer;
use common::Pubkey;
use signal_bus::{AlertType, LiveSignal, SignalBus, SignalKind, SignalSourceMeta, StrategyTag, SCHEMA_VERSION};
use signals::{Direction, QuoteArbSignal, WhaleEvent};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;

pub fn asset_to_pubkey(asset: &str) -> Pubkey {
    if asset.len() == 64 && asset.bytes().all(|b| b.is_ascii_hexdigit()) {
        let mut bytes = [0u8; 32];
        for (i, pair) in asset.as_bytes().chunks(2).enumerate() {
            if i >= 32 {
                break;
            }
            if pair.len() == 2 {
                let hi = hex_val(pair[0]);
                let lo = hex_val(pair[1]);
                if let (Some(h), Some(l)) = (hi, lo) {
                    bytes[i] = (h << 4) | l;
                }
            }
        }
        return Pubkey::new(bytes);
    }
    let mut bytes = [0u8; 32];
    for (i, b) in asset.bytes().enumerate() {
        bytes[i % 32] ^= b;
    }
    Pubkey::new(bytes)
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub fn live_to_whale_event(live: &LiveSignal) -> Option<WhaleEvent> {
    match live.alert_type {
        Some(AlertType::WhaleFlow) | Some(AlertType::SmartMoney) => {}
        _ => return None,
    }
    let direction = match live.direction.as_deref() {
        Some("Short") => Direction::Short,
        Some("Neutral") => Direction::Neutral,
        _ => Direction::Long,
    };
    let strength = live.strength.unwrap_or(live.confidence);
    Some(WhaleEvent {
        timestamp_micros: live.timestamp_micros(),
        pool_address: asset_to_pubkey(&live.token_out),
        swap_amount_usd: live.size_usd.unwrap_or(0.0),
        direction,
        profitability_score: live.confidence.max(strength),
    })
}

pub async fn push_live_signal(
    external: &RwLock<ExternalIngestionBuffer>,
    live: &LiveSignal,
) {
    if let Some(whale) = live_to_whale_event(live) {
        external
            .write()
            .await
            .push_whale(live.timestamp_micros(), whale);
        debug!(
            layer = %live.source.layer,
            alert = ?live.alert_type,
            "bridged live signal to autonomous whale ingestion"
        );
    }
}

/// Publish a quote-arb opportunity onto the shared signal bus.
pub async fn publish_quote_arb_signal(bus: &SignalBus, sig: &QuoteArbSignal) {
    let signal_id = format!("qarb-{}-{}", sig.pair_label, sig.detected_at_ms);
    let live = LiveSignal {
        v: SCHEMA_VERSION,
        signal_id: signal_id.clone(),
        kind: SignalKind::Engine,
        source: SignalSourceMeta {
            layer: "quote_arb".into(),
            dex: "jupiter".into(),
            slot: 0,
            wallet_label: None,
        },
        pair: sig.pair_label.clone(),
        token_in: sig.input_mint.clone(),
        token_out: sig.output_mint.clone(),
        timestamp_ms: sig.detected_at_ms,
        tx_id: signal_id,
        price: sig.expected_pnl_usd,
        size: sig.expected_pnl_usd.abs(),
        confidence: (sig.edge_bps as f64 / 10_000.0).clamp(0.0, 1.0),
        wallet: String::new(),
        strength: Some((sig.edge_bps as f64 / 10_000.0).clamp(0.0, 1.0)),
        size_usd: Some(sig.expected_pnl_usd.abs()),
        alert_type: None,
        strategy_tag: Some(StrategyTag::Informational),
        explanation: Some(format!(
            "Quote arb {} edge={}bps route_div={}bps",
            sig.pair_label, sig.edge_bps, sig.route_divergence_bps
        )),
        direction: Some("Long".into()),
        dedup_key: format!("quote_arb:{}:{}", sig.input_mint, sig.detected_at_ms),
    };
    bus.publish(live).await;
}

fn should_bridge(live: &LiveSignal) -> bool {
    live.source.layer == "intelligence"
        || matches!(
            live.alert_type,
            Some(AlertType::WhaleFlow) | Some(AlertType::SmartMoney)
        )
}

/// Subscribes to the shared signal bus and forwards whale/smart-money alerts.
pub fn spawn_signal_bus_bridge(
    bus: Arc<SignalBus>,
    external: Arc<RwLock<ExternalIngestionBuffer>>,
    cancel: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        for live in bus.replay().await {
            if should_bridge(&live) {
                push_live_signal(&external, &live).await;
            }
        }

        let mut sub = bus.subscribe();
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                msg = sub.recv() => {
                    match msg {
                        Ok(live) => push_live_signal(&external, &live).await,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            }
        }
    })
}
