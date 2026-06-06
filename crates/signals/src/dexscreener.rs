//! DexScreener free API — new pairs and volume spikes.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use config::DataSourcesConfig;
use ratelimit::{ApiSource, RateLimiter};
use tracing::warn;

use crate::external_store::{ExternalSignalStore, NewTokenSignal, WhaleActivitySignal};

const NEW_PAIR_MAX_AGE_SECS: u64 = 600;
const MIN_LIQUIDITY_USD: f64 = 5_000.0;

pub fn spawn_dexscreener_poller(
    store: ExternalSignalStore,
    data_sources: Arc<DataSourcesConfig>,
    limiter: Arc<RateLimiter>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let interval = limiter.poll_interval(ApiSource::DexScreener);
            if !limiter.try_acquire(ApiSource::DexScreener) {
                tokio::time::sleep(interval).await;
                continue;
            }

            let url = format!("{}/search?q=SOL", data_sources.dexscreener_base);
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        process_new_pairs(&store, &body);
                    }
                }
                Ok(resp) if resp.status().as_u16() == 429 => {
                    limiter.on_rate_limited(ApiSource::DexScreener);
                }
                Ok(resp) => warn!(status = %resp.status(), "dexscreener search failed"),
                Err(e) => warn!(error = %e, "dexscreener request error"),
            }

            tokio::time::sleep(interval).await;
        }
    });
}

pub fn spawn_volume_spike_poller(
    store: ExternalSignalStore,
    data_sources: Arc<DataSourcesConfig>,
    limiter: Arc<RateLimiter>,
    watch_mints: Arc<Vec<String>>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let interval = limiter.poll_interval(ApiSource::DexScreener).max(std::time::Duration::from_secs(30));
            tokio::time::sleep(interval).await;

            if !limiter.try_acquire(ApiSource::DexScreener) {
                continue;
            }

            let mut spikes = HashSet::new();
            for mint in watch_mints.iter().take(10) {
                let url = format!("{}/tokens/{}", data_sources.dexscreener_base, mint);
                let Ok(resp) = client.get(&url).send().await else {
                    continue;
                };
                if resp.status().as_u16() == 429 {
                    limiter.on_rate_limited(ApiSource::DexScreener);
                    break;
                }
                if !resp.status().is_success() {
                    continue;
                }
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    if let Some(sig) = parse_volume_spike(mint, &body) {
                        spikes.insert(sig.mint);
                    }
                }
            }
            store.set_volume_spikes(spikes);
        }
    });
}

fn process_new_pairs(store: &ExternalSignalStore, body: &serde_json::Value) {
    let now_ms = unix_now_ms();
    let pairs = body
        .pointer("/pairs")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    for pair in pairs {
        let chain = pair.get("chainId").and_then(|c| c.as_str()).unwrap_or("");
        if chain != "solana" {
            continue;
        }
        let created = pair
            .get("pairCreatedAt")
            .and_then(|c| c.as_u64())
            .unwrap_or(0);
        if now_ms.saturating_sub(created) > NEW_PAIR_MAX_AGE_SECS * 1000 {
            continue;
        }
        let liq = pair
            .pointer("/liquidity/usd")
            .and_then(|l| l.as_f64())
            .unwrap_or(0.0);
        if liq < MIN_LIQUIDITY_USD {
            continue;
        }
        let mint = pair
            .pointer("/baseToken/address")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_owned();
        if mint.is_empty() {
            continue;
        }

        let signal = NewTokenSignal {
            mint: mint.clone(),
            pair_address: pair
                .get("pairAddress")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_owned(),
            liquidity_usd: liq,
            volume_24h: pair
                .pointer("/volume/h24")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            created_at_ms: created,
        };
        store.push_new_token(signal);
    }
}

fn parse_volume_spike(mint: &str, body: &serde_json::Value) -> Option<WhaleActivitySignal> {
    let pair = body.pointer("/pairs/0")?;
    let h1_pct = pair
        .pointer("/priceChange/h1")
        .and_then(|p| p.as_f64())
        .unwrap_or(0.0);
    let vol_h1 = pair
        .pointer("/volume/h1")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    if h1_pct > 20.0 && vol_h1 > 50_000.0 {
        Some(WhaleActivitySignal {
            mint: mint.to_owned(),
            price_change_h1_pct: h1_pct,
            volume_h1_usd: vol_h1,
        })
    } else {
        None
    }
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Rugcheck filter — cache-friendly; call before acting on new tokens.
pub async fn rugcheck_passes(
    store: &ExternalSignalStore,
    data_sources: &DataSourcesConfig,
    limiter: &RateLimiter,
    mint: &str,
    client: &reqwest::Client,
) -> bool {
    if store.is_rugcheck_blocked(mint) {
        return false;
    }
    if !limiter.try_acquire(ApiSource::Rugcheck) {
        return true;
    }

    let url = format!("{}/tokens/{}/report", data_sources.rugcheck_base, mint);
    let Ok(resp) = client.get(&url).send().await else {
        return true;
    };
    if resp.status().as_u16() == 429 {
        limiter.on_rate_limited(ApiSource::Rugcheck);
        return true;
    }
    let Ok(body) = resp.json::<serde_json::Value>().await else {
        return true;
    };

    let score = body.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);
    let risks = body
        .get("risks")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    let bad_risk = risks.iter().any(|r| {
        r.as_str()
            .map(|s| s.contains("freeze_authority") || s.contains("no_lp_locked"))
            .unwrap_or(false)
    });

    let ok = score >= 700.0 && !bad_risk;
    if !ok {
        store.set_rugcheck_blocked(mint, true);
    }
    ok
}
