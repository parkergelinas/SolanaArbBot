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

/// Polls Jupiter's price API for aggregated SOL/USDC spot prices and emits
/// synthetic per-DEX prices to arb detection.  Bridges the gap between mock
/// feed and a real Geyser stream; useful when `GEYSER_GRPC_ENDPOINT` is set
/// but the `yellowstone` feature is not compiled in.
///
/// Activated by `GEYSER_GRPC_ENDPOINT` env var being present.
/// Poll interval defaults to `ARB_MOCK_INTERVAL_MS` (clamped to ≥ 1 000 ms).
pub fn spawn_live_price_poll(config: Arc<ArbConfig>, price_tx: Sender<PoolPrice>) {
    let interval_ms = config.mock_interval_ms.max(1_000);
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(_) => return,
        };
        rt.block_on(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let sol_mint = "So11111111111111111111111111111111111111112";
            let url =
                format!("https://lite-api.jup.ag/price/v2?ids={sol_mint}&vsToken=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

            info!(poll_ms = interval_ms, "live Jupiter price poll started (Geyser stub fallback)");

            loop {
                if let Ok(resp) = client.get(&url).send().await {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        if let Some(price) = body
                            .pointer(&format!("/data/{sol_mint}/price"))
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<f64>().ok())
                            .or_else(|| {
                                body.pointer(&format!("/data/{sol_mint}/price"))
                                    .and_then(|v| v.as_f64())
                            })
                        {
                            // Emit prices for each tracked DEX with a tiny synthetic skew
                            // so the detection engine sees cross-DEX spread candidates.
                            // In live Geyser mode these would be real per-pool prices.
                            let dexes = [("raydium", 0.0f64), ("orca", 0.001), ("meteora", -0.0005)];
                            let ts = unix_ms();
                            for (dex, skew) in dexes {
                                let pp = PoolPrice {
                                    dex: dex.into(),
                                    token_a: SOL.into(),
                                    token_b: USDC.into(),
                                    price: price * (1.0 + skew),
                                    liquidity: 500_000.0,
                                    timestamp: ts,
                                };
                                if price_tx.send(pp).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
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
