//! Birdeye public API — top movers and whale wallet snapshots.

use std::collections::HashSet;
use std::sync::Arc;

use config::DataSourcesConfig;
use ratelimit::{ApiSource, RateLimiter};
use tracing::{debug, warn};

use crate::external_store::ExternalSignalStore;

/// Known whale / arb wallets to poll (public portfolio endpoint).
pub const DEFAULT_WHALE_WALLETS: &[&str] = &[
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
    "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQjg",
    "DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK",
];

pub fn spawn_birdeye_top_movers_poller(
    store: ExternalSignalStore,
    data_sources: Arc<DataSourcesConfig>,
    limiter: Arc<RateLimiter>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let interval = limiter.poll_interval(ApiSource::Birdeye).max(std::time::Duration::from_secs(60));
            if !limiter.try_acquire(ApiSource::Birdeye) {
                tokio::time::sleep(interval).await;
                continue;
            }

            let url = format!(
                "{}/public/tokenlist?sort_by=v24hUSD&sort_type=desc&limit=20",
                data_sources.birdeye_base
            );

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        let mints = parse_top_movers(&body);
                        store.set_birdeye_top(mints);
                    }
                }
                Ok(resp) if resp.status().as_u16() == 429 => {
                    limiter.on_rate_limited(ApiSource::Birdeye);
                }
                Ok(resp) => warn!(status = %resp.status(), "birdeye tokenlist failed"),
                Err(e) => warn!(error = %e, "birdeye request error"),
            }

            tokio::time::sleep(interval).await;
        }
    });
    debug!("birdeye top-movers poller started");
}

pub fn spawn_birdeye_whale_poller(
    data_sources: Arc<DataSourcesConfig>,
    limiter: Arc<RateLimiter>,
    wallets: Arc<Vec<String>>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let interval = std::time::Duration::from_secs(30);
            tokio::time::sleep(interval).await;

            if !limiter.try_acquire(ApiSource::Birdeye) {
                continue;
            }

            for wallet in wallets.iter() {
                let url = format!(
                    "{}/public/portfolio?wallet={}",
                    data_sources.birdeye_base, wallet
                );
                if let Ok(resp) = client.get(&url).send().await {
                    if resp.status().as_u16() == 429 {
                        limiter.on_rate_limited(ApiSource::Birdeye);
                        break;
                    }
                    if resp.status().is_success() {
                        debug!(wallet = %wallet, "birdeye whale portfolio snapshot fetched");
                    }
                }
            }
        }
    });
}

fn parse_top_movers(body: &serde_json::Value) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Some(items) = body.pointer("/data/tokens").and_then(|t| t.as_array()) {
        for item in items {
            if let Some(addr) = item.get("address").and_then(|a| a.as_str()) {
                out.insert(addr.to_owned());
            }
        }
    }
    out
}

/// Fetch Jupiter strict token list once at startup.
pub async fn load_jupiter_verified(store: &ExternalSignalStore) {
    let client = reqwest::Client::new();
    let url = "https://token.jup.ag/strict";
    if let Ok(resp) = client.get(url).send().await {
        if let Ok(list) = resp.json::<Vec<serde_json::Value>>().await {
            let mints: HashSet<String> = list
                .iter()
                .filter_map(|t| t.get("address").and_then(|a| a.as_str()))
                .map(str::to_owned)
                .collect();
            store.set_jupiter_verified(mints);
        }
    }
}
