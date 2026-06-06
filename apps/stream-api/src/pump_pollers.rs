//! Pump.fun live stream — launches, trades, and DexScreener curve momentum → WebSocket.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Sender;
use ingestion::{
    spawn_pump_trade_ingest, spawn_sniper_ingest, PoolCreationEvent, PoolCreationSource,
    PumpTradeEvent, PumpTradeSide, SOL_MINT,
};
use signals::ScannerStore;
use tracing::info;

use crate::contracts::{Dex, Signal, SignalKind, SwapEvent, TokenPrice, WSMessage, SCHEMA_VERSION};

/// Wire pump.fun ingestion into the stream pipeline.
pub fn spawn_pump_pollers(
    swap_tx: Sender<SwapEvent>,
    ws_tx: Sender<WSMessage>,
    scanners: ScannerStore,
) {
    let data_sources = Arc::new(
        config::ConfigHandle::load()
            .map(|h| h.data_sources.clone())
            .unwrap_or_default(),
    );

    if data_sources.helius_api_key.is_empty() {
        info!("pump stream pollers: no HELIUS key — scanner fanout only");
    } else {
        spawn_launch_ingest(ws_tx.clone(), Arc::clone(&data_sources));
        spawn_trade_ingest(swap_tx, Arc::clone(&data_sources));
    }

    spawn_scanner_fanout(ws_tx, scanners);
}

fn spawn_launch_ingest(ws_tx: Sender<WSMessage>, data_sources: Arc<config::DataSourcesConfig>) {
    let (create_tx, create_rx) = crossbeam_channel::unbounded();
    spawn_sniper_ingest(create_tx, data_sources);
    let signal_seq = Arc::new(AtomicU64::new(0));

    std::thread::spawn(move || {
        while let Ok(ev) = create_rx.recv() {
            if ev.source != PoolCreationSource::PumpFun {
                continue;
            }
            let _ = ws_tx.send(launch_to_signal(&ev, &signal_seq));
        }
    });
    info!("pump launch ingest started (Helius Create)");
}

fn spawn_trade_ingest(swap_tx: Sender<SwapEvent>, data_sources: Arc<config::DataSourcesConfig>) {
    let (trade_tx, trade_rx) = crossbeam_channel::unbounded();
    spawn_pump_trade_ingest(trade_tx, data_sources);

    std::thread::spawn(move || {
        while let Ok(ev) = trade_rx.recv() {
            let _ = swap_tx.send(trade_to_swap(&ev));
        }
    });
    info!("pump trade ingest started (Helius Buy/Sell)");
}

fn launch_to_signal(ev: &PoolCreationEvent, seq: &AtomicU64) -> WSMessage {
    let id = seq.fetch_add(1, Ordering::Relaxed);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    WSMessage::Signal(Signal {
        v: SCHEMA_VERSION,
        signal_id: format!("pump_launch_{id}"),
        mint: ev.token_mint.clone(),
        kind: SignalKind::Momentum,
        strength: 0.85,
        confidence: 0.9,
        timestamp_ms: ts,
        detail: Some(format!(
            "pump_launch:creator={};liq_sol={:.4};sig={}",
            ev.creator_wallet, ev.initial_liquidity_sol, ev.signature
        )),
    })
}

fn trade_to_swap(ev: &PumpTradeEvent) -> SwapEvent {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let sol_amount = if ev.sol_lamports > 0 {
        ev.sol_lamports.to_string()
    } else {
        "100000000".to_string()
    };

    let (token_in, token_out, amount_in, amount_out) = match ev.side {
        PumpTradeSide::Buy => (
            SOL_MINT.to_owned(),
            ev.token_mint.clone(),
            sol_amount,
            "0".to_owned(),
        ),
        PumpTradeSide::Sell => (
            ev.token_mint.clone(),
            SOL_MINT.to_owned(),
            "0".to_owned(),
            sol_amount,
        ),
    };

    SwapEvent::new_v1(
        &ev.signature,
        Dex::Pump,
        token_in,
        token_out,
        amount_in,
        amount_out,
        &ev.wallet,
        ev.slot,
        ts,
    )
}

/// Fan DexScreener pump scanner hits to WebSocket as curve momentum signals + prices.
fn spawn_scanner_fanout(ws_tx: Sender<WSMessage>, scanners: ScannerStore) {
    tokio::spawn(async move {
        let mut last_polled: u64 = 0;
        let signal_seq = Arc::new(AtomicU64::new(0));

        loop {
            tokio::time::sleep(Duration::from_secs(12)).await;
            let (hits, meta) = scanners.pump_snapshot();
            if meta.polled_at_ms == 0 || meta.polled_at_ms == last_polled {
                continue;
            }
            last_polled = meta.polled_at_ms;

            for hit in hits.iter().take(15) {
                let id = signal_seq.fetch_add(1, Ordering::Relaxed);
                let _ = ws_tx.send(WSMessage::Signal(Signal {
                    v: SCHEMA_VERSION,
                    signal_id: format!("pump_curve_{id}"),
                    mint: hit.mint.clone(),
                    kind: SignalKind::Momentum,
                    strength: (hit.momentum_score as f64) / 100.0,
                    confidence: 0.75,
                    timestamp_ms: hit.detected_at_ms,
                    detail: Some(format!(
                        "pump_curve:symbol={};grad_pct={:.1};buys_m5={};vol_m5={:.0};score={}",
                        hit.symbol, hit.graduation_pct, hit.buys_m5, hit.volume_m5_usd, hit.momentum_score
                    )),
                }));

                if hit.market_cap_usd > 0.0 {
                    let price = hit.market_cap_usd / 1_000_000_000.0;
                    let _ = ws_tx.send(WSMessage::TokenPrice(TokenPrice {
                        v: SCHEMA_VERSION,
                        mint: hit.mint.clone(),
                        price_usd: price.max(1e-12),
                        slot: 0,
                        timestamp_ms: hit.detected_at_ms,
                    }));
                }
            }

            if !hits.is_empty() {
                info!(count = hits.len(), "pump scanner fanout to stream");
            }
        }
    });
}
