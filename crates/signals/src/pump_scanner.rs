//! Pump.fun early momentum scanner — DexScreener poller.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use config::DataSourcesConfig;
use ratelimit::{ApiSource, RateLimiter};
use crate::scanner_store::{PumpMomentumHit, ScannerMeta, ScannerStore};

const GRADUATION_MCAP: f64 = 69_000.0;

pub fn spawn_pump_scanner(
    store: ScannerStore,
    data_sources: Arc<DataSourcesConfig>,
    limiter: Arc<RateLimiter>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let interval = limiter
                .poll_interval(ApiSource::DexScreener)
                .max(std::time::Duration::from_secs(20));
            tokio::time::sleep(interval).await;

            if !limiter.try_acquire(ApiSource::DexScreener) {
                continue;
            }

            let url = format!("{}/search?q=pumpfun", data_sources.dexscreener_base);
            let Ok(resp) = client.get(&url).send().await else {
                continue;
            };
            if !resp.status().is_success() {
                if resp.status().as_u16() == 429 {
                    limiter.on_rate_limited(ApiSource::DexScreener);
                }
                continue;
            }

            let Ok(body) = resp.json::<serde_json::Value>().await else {
                continue;
            };

            let now_ms = unix_now_ms();
            let mut hits = Vec::new();

            if let Some(pairs) = body.get("pairs").and_then(|p| p.as_array()) {
                for pair in pairs {
                    if pair.get("chainId").and_then(|c| c.as_str()) != Some("solana") {
                        continue;
                    }
                    let dex = pair.get("dexId").and_then(|d| d.as_str()).unwrap_or("");
                    if !dex.to_lowercase().contains("pump") {
                        continue;
                    }
                    if let Some(hit) = parse_pump_pair(pair, now_ms) {
                        hits.push(hit);
                    }
                }
            }

            hits.sort_by(|a, b| b.momentum_score.cmp(&a.momentum_score));
            hits.truncate(25);

            store.set_pump(
                hits,
                ScannerMeta {
                    source: "dexscreener".to_owned(),
                    degraded: false,
                    message: None,
                    polled_at_ms: now_ms,
                },
            );
        }
    });
}

fn parse_pump_pair(pair: &serde_json::Value, now_ms: u64) -> Option<PumpMomentumHit> {
    let mint = pair
        .pointer("/baseToken/address")
        .and_then(|m| m.as_str())?;
    let symbol = pair
        .pointer("/baseToken/symbol")
        .and_then(|s| s.as_str())
        .unwrap_or("???");
    let created = pair.get("pairCreatedAt").and_then(|c| c.as_u64()).unwrap_or(0);
    let age_min = if created > 0 {
        ((now_ms.saturating_sub(created)) / 60_000) as u32
    } else {
        9999
    };
    if age_min > 720 {
        return None;
    }

    let vol_m5 = pair.pointer("/volume/m5").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let buys_m5 = pair
        .pointer("/txns/m5/buys")
        .and_then(|b| b.as_u64())
        .unwrap_or(0) as u32;
    if vol_m5 < 2_000.0 && buys_m5 < 5 {
        return None;
    }

    let mcap = pair.get("marketCap").and_then(|m| m.as_f64()).unwrap_or(0.0);
    let grad = (mcap / GRADUATION_MCAP * 100.0).min(100.0);
    let chg_m5 = pair
        .pointer("/priceChange/m5")
        .and_then(|p| p.as_f64())
        .unwrap_or(0.0);

    let mut score = 0u32;
    if age_min <= 30 {
        score += 30;
    } else if age_min <= 120 {
        score += 20;
    }
    if vol_m5 >= 5_000.0 {
        score += 15;
    }
    if buys_m5 >= 8 {
        score += 15;
    }
    if chg_m5 > 3.0 {
        score += 10;
    }
    if grad >= 40.0 && grad < 95.0 {
        score += 10;
    }
    if score < 35 {
        return None;
    }

    let pair_addr = pair.get("pairAddress").and_then(|p| p.as_str()).unwrap_or("");
    let dex_url = pair
        .get("url")
        .and_then(|u| u.as_str())
        .unwrap_or("")
        .to_owned();

    Some(PumpMomentumHit {
        id: format!("{mint}-{pair_addr}"),
        mint: mint.to_owned(),
        symbol: symbol.to_owned(),
        age_minutes: age_min,
        volume_m5_usd: vol_m5,
        buys_m5,
        market_cap_usd: mcap,
        graduation_pct: grad,
        momentum_score: score.min(100),
        detected_at_ms: now_ms,
        pump_url: format!("https://pump.fun/coin/{mint}"),
        dex_url,
    })
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
