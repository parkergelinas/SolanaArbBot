//! Jupiter Price API poller — v3 (`api.jup.ag/price/v3`), v2 legacy parse supported.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{MarketEvent, PriceUpdate, Pubkey};
use config::DataSourcesConfig;
use events::EventBus;
use ratelimit::{ApiSource, RateLimiter};
use tracing::{debug, warn};

/// Current Jupiter Price API (v2 `price.jup.ag` retired — DNS no longer resolves).
pub const JUPITER_PRICE_V3_URL: &str = "https://api.jup.ag/price/v3";

const MIN_DELTA_PCT: f64 = 0.05;
const LEGACY_JUPITER_HOST: &str = "price.jup.ag";

/// Shared mint list to poll (base58 strings).
pub type MintWatchlist = Arc<Vec<String>>;

/// Poll Jupiter prices and publish `MarketEvent::PriceUpdate` when delta > 0.05%.
pub fn spawn_jupiter_poller(
    bus: EventBus,
    data_sources: Arc<DataSourcesConfig>,
    mints: MintWatchlist,
    limiter: Arc<RateLimiter>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let cache: Arc<Mutex<HashMap<String, f64>>> = Arc::new(Mutex::new(HashMap::new()));

        loop {
            let interval = limiter.poll_interval(ApiSource::Jupiter);
            if !limiter.try_acquire(ApiSource::Jupiter) {
                tokio::time::sleep(interval).await;
                continue;
            }

            let batch: Vec<String> = mints.iter().take(50).cloned().collect();
            if batch.is_empty() {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            let base = normalize_jupiter_price_url(&data_sources.jupiter_price);
            let url = format!("{}?ids={}", base, batch.join(","));

            match jupiter_get(&client, &url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        publish_prices(&bus, &cache, &body);
                    }
                }
                Ok(resp) if resp.status().as_u16() == 429 => {
                    limiter.on_rate_limited(ApiSource::Jupiter);
                }
                Ok(resp) => {
                    warn!(status = %resp.status(), "jupiter price poll failed");
                }
                Err(e) => warn!(error = %e, "jupiter price request error"),
            }

            tokio::time::sleep(interval).await;
        }
    });
    debug!("jupiter price poller started");
}

/// Normalize configured URL — auto-upgrade retired `price.jup.ag` host to v3.
pub fn normalize_jupiter_price_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.contains(LEGACY_JUPITER_HOST) {
        warn!(
            configured = %trimmed,
            migrated = JUPITER_PRICE_V3_URL,
            "price.jup.ag is deprecated; using Jupiter Price API v3"
        );
        return JUPITER_PRICE_V3_URL.to_owned();
    }
    trimmed.to_owned()
}

fn jupiter_get(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    let mut req = client.get(url);
    if let Ok(key) = std::env::var("JUPITER_API_KEY") {
        let key = key.trim();
        if !key.is_empty() {
            req = req.header("x-api-key", key);
        }
    }
    req
}

/// Parse Jupiter Price API v2 (`data.{mint}.price`) or v3 (`{mint}.usdPrice`).
pub fn extract_jupiter_prices(body: &serde_json::Value) -> Vec<(String, f64)> {
    let mut out = Vec::new();

    if let Some(data) = body.get("data").and_then(|d| d.as_object()) {
        for (mint, entry) in data {
            if let Some(price) = entry.get("price").and_then(|p| p.as_f64()) {
                if price.is_finite() && price > 0.0 {
                    out.push((mint.clone(), price));
                }
            }
        }
        return out;
    }

    if let Some(obj) = body.as_object() {
        for (mint, entry) in obj {
            if let Some(price) = entry.get("usdPrice").and_then(|p| p.as_f64()) {
                if price.is_finite() && price > 0.0 {
                    out.push((mint.clone(), price));
                }
            }
        }
    }

    out
}

fn publish_prices(
    bus: &EventBus,
    cache: &Mutex<HashMap<String, f64>>,
    body: &serde_json::Value,
) {
    let mut guard = cache.lock().expect("price cache");
    for (mint, price) in extract_jupiter_prices(body) {
        let prev = guard.get(&mint).copied().unwrap_or(price);
        let delta_pct = ((price - prev) / prev.max(1e-12)).abs() * 100.0;
        guard.insert(mint.clone(), price);

        if delta_pct < MIN_DELTA_PCT && guard.len() > 1 {
            continue;
        }

        let mint_bytes = bs58::decode(&mint).into_vec().unwrap_or_default();
        if mint_bytes.len() != 32 {
            continue;
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&mint_bytes);

        let _ = bus.publish(MarketEvent::PriceUpdate(PriceUpdate {
            mint: Pubkey::new(arr),
            price_usd: price,
            source: "jupiter".to_owned(),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v3_response() {
        let body = serde_json::json!({
            "So11111111111111111111111111111111111111112": {
                "usdPrice": 63.42
            }
        });
        let prices = extract_jupiter_prices(&body);
        assert_eq!(prices.len(), 1);
        assert!((prices[0].1 - 63.42).abs() < f64::EPSILON);
    }

    #[test]
    fn migrates_legacy_host() {
        assert_eq!(
            normalize_jupiter_price_url("https://price.jup.ag/v2/price"),
            JUPITER_PRICE_V3_URL
        );
    }
}
