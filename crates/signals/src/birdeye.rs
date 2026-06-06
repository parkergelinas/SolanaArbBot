//! Birdeye public API — top movers and whale wallet snapshots.

use std::collections::HashSet;
use std::sync::Arc;

use config::DataSourcesConfig;
use ratelimit::{ApiSource, RateLimiter};
use tracing::{debug, warn};

use crate::external_store::ExternalSignalStore;
use crate::whale_watcher::short_wallet;

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
                        debug!(wallet = %short_wallet(wallet), "birdeye whale portfolio snapshot fetched");
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

/// Fetch Jupiter verified tokens once at startup (Tokens API v2).
pub async fn load_jupiter_verified(store: &ExternalSignalStore) {
    load_jupiter_verified_from_base(store, pricing::JUPITER_TOKENS_V2_BASE).await;
}

/// Fetch verified tokens from a configurable Tokens API v2 base URL.
pub async fn load_jupiter_verified_from_base(store: &ExternalSignalStore, tokens_base: &str) {
    let client = reqwest::Client::new();
    match pricing::fetch_verified_tokens(&client, tokens_base, 5000).await {
        Ok(tokens) => {
            let mints: HashSet<String> = tokens
                .into_iter()
                .filter(|t| {
                    t.tags
                        .iter()
                        .any(|tag| tag.eq_ignore_ascii_case("verified"))
                })
                .map(|t| t.id)
                .collect();
            store.set_jupiter_verified(mints);
        }
        Err(e) => {
            tracing::warn!(error = %e, "jupiter tokens v2 verified load failed");
        }
    }
}
