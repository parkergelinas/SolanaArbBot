//! Jupiter Price API v2 poller — free, no auth.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{MarketEvent, PriceUpdate, Pubkey};
use config::DataSourcesConfig;
use events::EventBus;
use ratelimit::{ApiSource, RateLimiter};
use tracing::{debug, warn};

const MIN_DELTA_PCT: f64 = 0.05;

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

            let url = format!(
                "{}?ids={}",
                data_sources.jupiter_price.trim_end_matches('/'),
                batch.join(",")
            );

            match client.get(&url).send().await {
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
    let Some(data) = body.get("data").and_then(|d| d.as_object()) else {
        return;
    };

    let mut guard = cache.lock().expect("price cache");
    for (mint, entry) in data {
        let Some(price) = entry.get("price").and_then(|p| p.as_f64()) else {
            continue;
        };
        let prev = guard.get(mint).copied().unwrap_or(price);
        let delta_pct = ((price - prev) / prev.max(1e-12)).abs() * 100.0;
        guard.insert(mint.clone(), price);

        if delta_pct < MIN_DELTA_PCT && guard.len() > 1 {
            continue;
        }

        let mint_bytes = bs58::decode(mint).into_vec().unwrap_or_default();
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
