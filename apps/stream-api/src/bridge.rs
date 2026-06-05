//! Data-layer → signal-bus → stream-api market engine bridge.

use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use data_layer::types::PipelineEvent;
use signal_bus::{from_enriched, LiveSignal, SignalBus};
use tracing::info;

use crate::contracts::{Dex, SwapEvent};

fn parse_dex(dex: &str) -> Dex {
    match dex.to_lowercase().as_str() {
        "orca" => Dex::Orca,
        "jupiter" => Dex::Jupiter,
        _ => Dex::Raydium,
    }
}

/// Convert normalized live signal into stream wire `SwapEvent` (no frontend special-casing).
pub fn live_to_stream_swap(live: &LiveSignal) -> SwapEvent {
    let amount_in_lamports = (live.size * 1_000_000_000.0) as u64;
    let amount_out = if live.price > 0.0 {
        ((live.size * live.price) * 1_000_000.0) as u64
    } else {
        amount_in_lamports
    };

    SwapEvent::new_v1(
        &live.tx_id,
        parse_dex(&live.source.dex),
        &live.token_in,
        &live.token_out,
        amount_in_lamports.to_string(),
        amount_out.to_string(),
        &live.wallet,
        live.source.slot,
        live.timestamp_ms,
    )
}

/// Consumes data-layer pipeline output, publishes to signal-bus, forwards to market engine.
pub fn spawn_data_layer_bridge(
    pipeline_rx: Receiver<PipelineEvent>,
    bus: Arc<SignalBus>,
    swap_tx: Sender<SwapEvent>,
) {
    std::thread::spawn(move || {
        let replayed = bus.replay_blocking();
        if !replayed.is_empty() {
            info!(count = replayed.len(), "replaying signal-bus buffer on bridge start");
            for live in replayed {
                let _ = swap_tx.send(live_to_stream_swap(&live));
            }
        }

        while let Ok(ev) = pipeline_rx.recv() {
            let enriched = ev.enriched();
            let live = from_enriched(enriched);
            if let Some(accepted) = bus.publish_blocking(live) {
                let _ = swap_tx.send(live_to_stream_swap(&accepted));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use signal_bus::{SignalKind, SignalSourceMeta, SCHEMA_VERSION};

    #[test]
    fn live_maps_to_stream_swap_shape() {
        let live = LiveSignal {
            v: SCHEMA_VERSION,
            signal_id: "sig".into(),
            kind: SignalKind::Swap,
            source: SignalSourceMeta {
                layer: "data-layer".into(),
                dex: "raydium".into(),
                slot: 99,
                wallet_label: None,
            },
            pair: "SOL/USDC".into(),
            token_in: signal_bus::SOL_MINT.into(),
            token_out: signal_bus::USDC_MINT.into(),
            timestamp_ms: 1_700_000_000_000,
            tx_id: "sig".into(),
            price: 145.0,
            size: 1.0,
            confidence: 0.8,
            wallet: "w".into(),
            strength: None,
            size_usd: None,
            alert_type: None,
            strategy_tag: None,
            explanation: None,
            direction: None,
            dedup_key: "sig".into(),
        };
        let swap = live_to_stream_swap(&live);
        assert_eq!(swap.signature, "sig");
        assert_eq!(swap.slot, 99);
        assert_eq!(swap.dex, Dex::Raydium);
        assert_eq!(swap.v, SCHEMA_VERSION);
    }
}
