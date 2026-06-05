//! Price stream ingest — mock feed or upstream subscriber stub.

use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Sender;
use tracing::info;

use crate::config::ArbConfig;
use crate::latency::DetectionTracker;
use crate::normalizer::normalize;
use crate::pool_state::PoolStateEngine;
use crate::types::{canonical_pair, PoolPrice, unix_ms};

const SOL: &str = "So11111111111111111111111111111111111111112";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

pub fn spawn_mock_price_feed(config: Arc<ArbConfig>, price_tx: Sender<PoolPrice>) {
    let interval = config.mock_interval_ms;
    std::thread::spawn(move || {
        let dexes = ["raydium", "orca", "meteora"];
        let mut tick: u64 = 0;
        info!("mock price feed started ({interval}ms)");
        loop {
            tick += 1;
            let base = 145.0 + (tick % 7) as f64 * 0.1;
            for (i, dex) in dexes.iter().enumerate() {
                let skew = match i {
                    0 => 0.0,
                    1 => 0.6 + (tick % 3) as f64 * 0.15,
                    _ => -0.2 + (tick % 2) as f64 * 0.1,
                };
                let price = PoolPrice {
                    dex: (*dex).into(),
                    token_a: SOL.into(),
                    token_b: USDC.into(),
                    price: base + skew,
                    liquidity: 40_000.0 + i as f64 * 15_000.0,
                    timestamp: unix_ms(),
                };
                if price_tx.send(price).is_err() {
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(interval));
        }
    });
}

pub fn spawn_price_consumer(
    pool_engine: Arc<PoolStateEngine>,
    price_rx: crossbeam_channel::Receiver<PoolPrice>,
) {
    std::thread::spawn(move || {
        while let Ok(raw) = price_rx.recv() {
            let t0 = std::time::Instant::now();
            if let Ok(norm) = normalize(raw) {
                pool_engine.upsert(&norm);
                let mut lat = DetectionTracker::start();
                lat.record("normalize", t0);
                lat.record("pool", t0);
                let _ = lat.finish();
            }
        }
    });
}

pub fn spawn_batch_detector(
    config: Arc<ArbConfig>,
    pool_engine: Arc<PoolStateEngine>,
    detect_tx: Sender<()>,
) {
    let interval = config.batch_interval_ms;
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(interval));
            if detect_tx.send(()).is_err() {
                break;
            }
        }
    });
}

pub fn spawn_intelligence_stub(_tx: Sender<PoolPrice>) {
    info!("intelligence price stream stub — wire to data-layer swap feed");
}

/// Poll signal-hub swap events and derive pool prices (replaces mock when enabled).
pub fn spawn_signal_hub_price_feed(config: Arc<ArbConfig>, price_tx: Sender<PoolPrice>) {
    let hub = std::env::var("SIGNAL_HUB_URL")
        .or_else(|_| std::env::var("CONTROL_API_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:3001".into());
    let hub = hub.trim_end_matches('/').to_string();
    let poll_ms = config.mock_interval_ms.max(500);

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(_) => return,
        };
        rt.block_on(async move {
            let client = reqwest::Client::new();
            let url = format!("{hub}/api/live-signals?limit=100");
            info!(url = %url, "arb-engine signal-hub price feed started");
            loop {
                if let Ok(resp) = client.get(&url).send().await {
                    if resp.status().is_success() {
                        if let Ok(signals) = resp.json::<Vec<serde_json::Value>>().await {
                            for s in signals {
                                if s.get("kind").and_then(|k| k.as_str()) != Some("swap") {
                                    continue;
                                }
                                let dex = s
                                    .pointer("/source/dex")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("unknown");
                                let price = s.get("price").and_then(|p| p.as_f64()).unwrap_or(0.0);
                                let liquidity = s
                                    .get("size_usd")
                                    .and_then(|p| p.as_f64())
                                    .unwrap_or(10_000.0)
                                    * 4.0;
                                if price <= 0.0 {
                                    continue;
                                }
                                let pp = PoolPrice {
                                    dex: dex.into(),
                                    token_a: SOL.into(),
                                    token_b: USDC.into(),
                                    price,
                                    liquidity,
                                    timestamp: unix_ms(),
                                };
                                if price_tx.send(pp).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(poll_ms)).await;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_pair_canonical() {
        let pair = canonical_pair(SOL, USDC);
        assert!(pair.contains(SOL));
        assert!(pair.contains(USDC));
    }
}
