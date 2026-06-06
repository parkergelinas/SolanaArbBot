//! Solscan / DexScreener shitcoin whale researcher poller.

use std::sync::Arc;

use config::DataSourcesConfig;
use ratelimit::{ApiSource, RateLimiter};

use crate::scanner_store::{ScannerMeta, ScannerStore, ShitcoinWhaleHit};

const MAX_MCAP: f64 = 750_000.0;

pub fn spawn_solscan_researcher(
    store: ScannerStore,
    data_sources: Arc<DataSourcesConfig>,
    limiter: Arc<RateLimiter>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let interval = limiter
                .poll_interval(ApiSource::DexScreener)
                .max(std::time::Duration::from_secs(25));
            tokio::time::sleep(interval).await;

            if !limiter.try_acquire(ApiSource::DexScreener) {
                continue;
            }

            let url = format!("{}/search?q=solana+meme", data_sources.dexscreener_base);
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
            let solscan_key = std::env::var("SOLSCAN_API_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty());
            let degraded = solscan_key.is_none();

            let mut hits = Vec::new();
            if let Some(pairs) = body.get("pairs").and_then(|p| p.as_array()) {
                for pair in pairs.iter().take(30) {
                    if pair.get("chainId").and_then(|c| c.as_str()) != Some("solana") {
                        continue;
                    }
                    if let Some(hit) = parse_shitcoin_hit(pair, now_ms, degraded) {
                        hits.push(hit);
                    }
                }
            }

            hits.sort_by(|a, b| b.score.cmp(&a.score));
            hits.truncate(30);

            store.set_solscan(
                hits,
                ScannerMeta {
                    source: if degraded {
                        "dexscreener-inferred".to_owned()
                    } else {
                        "solscan+dex".to_owned()
                    },
                    degraded,
                    message: if degraded {
                        Some(
                            "Set SOLSCAN_API_KEY or HELIUS_API_KEY for wallet resolution.".to_owned(),
                        )
                    } else {
                        None
                    },
                    polled_at_ms: now_ms,
                },
            );
        }
    });
}

fn parse_shitcoin_hit(
    pair: &serde_json::Value,
    now_ms: u64,
    degraded: bool,
) -> Option<ShitcoinWhaleHit> {
    let mint = pair.pointer("/baseToken/address").and_then(|m| m.as_str())?;
    if mint == "So11111111111111111111111111111111111111112" {
        return None;
    }
    let symbol = pair
        .pointer("/baseToken/symbol")
        .and_then(|s| s.as_str())
        .unwrap_or("???");
    let mcap = pair.get("marketCap").and_then(|m| m.as_f64()).unwrap_or(0.0);
    let liq = pair.pointer("/liquidity/usd").and_then(|l| l.as_f64()).unwrap_or(0.0);
    let vol_m5 = pair.pointer("/volume/m5").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let buys_m5 = pair
        .pointer("/txns/m5/buys")
        .and_then(|b| b.as_u64())
        .unwrap_or(0);

    if mcap > MAX_MCAP || vol_m5 < 8_000.0 || buys_m5 < 12 {
        return None;
    }

    let mut score = 0u32;
    if mcap < MAX_MCAP {
        score += 25;
    }
    if liq < 80_000.0 {
        score += 15;
    }
    if vol_m5 >= 8_000.0 {
        score += 20;
    }
    if buys_m5 >= 12 {
        score += 20;
    }
    if score < 40 {
        return None;
    }

    let wallet = if degraded {
        "unknown — connect SOLSCAN_API_KEY".to_owned()
    } else {
        "pending-solscan-lookup".to_owned()
    };

    Some(ShitcoinWhaleHit {
        id: format!("{mint}-research"),
        wallet,
        token_mint: mint.to_owned(),
        token_symbol: symbol.to_owned(),
        amount_usd: vol_m5,
        market_cap_usd: mcap,
        liquidity_usd: liq,
        detected_at_ms: now_ms,
        source: if degraded {
            "dexscreener".to_owned()
        } else {
            "solscan".to_owned()
        },
        score: score.min(100),
        solscan_url: format!("https://solscan.io/token/{mint}"),
    })
}

fn unix_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
