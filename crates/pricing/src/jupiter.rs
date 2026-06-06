//! Jupiter Price API poller — v3 (`api.jup.ag/price/v3`), v2 legacy parse supported.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{MarketEvent, PriceUpdate, Pubkey};
use config::DataSourcesConfig;
use events::EventBus;
use ratelimit::{ApiSource, RateLimiter};
use tracing::{debug, warn};

use crate::jupiter_api::{extract_jupiter_prices, jupiter_get, normalize_jupiter_price_url};

const MIN_DELTA_PCT: f64 = 0.05;

pub use crate::jupiter_api::JUPITER_PRICE_V3_URL as PRICE_V3;

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
    use crate::jupiter_api::JUPITER_PRICE_V3_URL;

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
