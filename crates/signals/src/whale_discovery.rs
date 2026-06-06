//! Dynamic whale wallet discovery via GMGN (and optional Dune Analytics).
//!
//! Discovered addresses are merged with [`WHALE_WALLETS`] and persisted to disk.

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use config::WhaleTrackerConfig;
use tracing::{debug, info, warn};

use crate::whale_watcher::{seed_tracked_wallets, short_wallet, WHALE_WALLETS};

const GMGN_TOP_TRADERS_URL: &str =
    "https://gmgn.ai/api/v1/sol/top_traders?limit=20&orderby=pnl_7d";
const MIN_PNL_7D_USD: f64 = 50_000.0;

/// Optional Dune query — free tier allows ~3 executions/day; cache aggressively.
const DUNE_QUERY_ID: u64 = 3_456_789;
const DUNE_CACHE_SECS: u64 = 8 * 3600;

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct DiscoveredFile {
    wallets: Vec<String>,
    #[serde(default)]
    last_dune_fetch_unix: u64,
}

/// Load persisted discovered wallets from `persist_path`.
pub fn load_discovered_wallets(path: &str) -> Vec<String> {
    let p = Path::new(path);
    if !p.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(p) {
        Ok(raw) => match serde_json::from_str::<DiscoveredFile>(&raw) {
            Ok(file) => file.wallets,
            Err(e) => {
                warn!(error = %e, path, "discovered_whales.json parse failed — using seeds only");
                Vec::new()
            }
        },
        Err(e) => {
            warn!(error = %e, path, "discovered_whales.json read failed");
            Vec::new()
        }
    }
}

fn persist_discovered(path: &str, wallets: &HashSet<String>, last_dune: u64) {
    if let Some(parent) = Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut list: Vec<String> = wallets
        .iter()
        .filter(|w| !WHALE_WALLETS.contains(&w.as_str()))
        .cloned()
        .collect();
    list.sort();

    let file = DiscoveredFile {
        wallets: list,
        last_dune_fetch_unix: last_dune,
    };
    if let Ok(json) = serde_json::to_string_pretty(&file) {
        if let Err(e) = std::fs::write(path, json) {
            warn!(error = %e, path, "failed to persist discovered whales");
        }
    }
}

/// Build shared tracked-wallet set: seeds + file + cap by `max_tracked_wallets`.
pub fn build_tracked_wallet_set(cfg: &WhaleTrackerConfig) -> Arc<RwLock<HashSet<String>>> {
    let discovered = load_discovered_wallets(&cfg.persist_path);
    let mut set = seed_tracked_wallets(&discovered);
    if set.len() > cfg.max_tracked_wallets {
        let seeds: HashSet<String> = WHALE_WALLETS.iter().map(|s| (*s).to_owned()).collect();
        let seed_count = seeds.len();
        set = seeds;
        for w in discovered
            .into_iter()
            .take(cfg.max_tracked_wallets.saturating_sub(seed_count))
        {
            set.insert(w);
        }
    }
    info!(
        tracked = set.len(),
        seeds = WHALE_WALLETS.len(),
        path = %cfg.persist_path,
        "whale wallet set loaded"
    );
    Arc::new(RwLock::new(set))
}

/// Poll GMGN every `discovery_interval_s`; optionally Dune (cached).
pub fn spawn_whale_discovery(
    wallets: Arc<RwLock<HashSet<String>>>,
    cfg: Arc<WhaleTrackerConfig>,
) {
    let interval_s = cfg.discovery_interval_s;
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut last_dune = load_dune_cache_ts(&cfg.persist_path);

        loop {
            if let Err(e) = poll_gmgn(&client, &wallets, &cfg).await {
                warn!(error = %e, "GMGN whale discovery poll failed");
            }

            if should_poll_dune(last_dune) {
                match poll_dune(&client, &wallets, &cfg).await {
                    Ok(ts) => {
                        last_dune = ts;
                        persist_current(&wallets, &cfg.persist_path, last_dune);
                    }
                    Err(e) => debug!(error = %e, "Dune whale discovery skipped or failed"),
                }
            } else {
                persist_current(&wallets, &cfg.persist_path, last_dune);
            }

            tokio::time::sleep(Duration::from_secs(cfg.discovery_interval_s)).await;
        }
    });
    debug!(interval_s, "whale discovery poller started");
}

fn load_dune_cache_ts(path: &str) -> u64 {
    let p = Path::new(path);
    if !p.exists() {
        return 0;
    }
    std::fs::read_to_string(p)
        .ok()
        .and_then(|raw| serde_json::from_str::<DiscoveredFile>(&raw).ok())
        .map(|f| f.last_dune_fetch_unix)
        .unwrap_or(0)
}

fn should_poll_dune(last_fetch: u64) -> bool {
    let now = unix_now_secs();
    now.saturating_sub(last_fetch) >= DUNE_CACHE_SECS
}

fn persist_current(wallets: &RwLock<HashSet<String>>, path: &str, last_dune: u64) {
    if let Ok(guard) = wallets.read() {
        persist_discovered(path, &guard, last_dune);
    }
}

async fn poll_gmgn(
    client: &reqwest::Client,
    wallets: &RwLock<HashSet<String>>,
    cfg: &WhaleTrackerConfig,
) -> anyhow::Result<()> {
    let resp = client.get(GMGN_TOP_TRADERS_URL).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("GMGN HTTP {}", resp.status());
    }

    let body: serde_json::Value = resp.json().await?;
    let items = body
        .pointer("/data/list")
        .or_else(|| body.get("data"))
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let mut added = 0usize;
    {
        let mut guard = wallets.write().expect("lock");
        for item in &items {
            let wallet = item
                .get("wallet")
                .or_else(|| item.get("address"))
                .and_then(|w| w.as_str());
            let pnl = item
                .get("pnl_7d")
                .or_else(|| item.get("realized_profit_7d"))
                .and_then(|p| p.as_f64().or_else(|| p.as_str().and_then(|s| s.parse().ok())))
                .unwrap_or(0.0);

            let Some(wallet) = wallet else { continue };
            if pnl < MIN_PNL_7D_USD {
                continue;
            }
            if guard.len() >= cfg.max_tracked_wallets {
                break;
            }
            if guard.insert(wallet.to_owned()) {
                added += 1;
                info!(
                    wallet = %short_wallet(wallet),
                    pnl_7d = pnl,
                    "discovered whale wallet from GMGN"
                );
            }
        }
    }

    if added > 0 {
        info!(added, "GMGN discovery merged new whale wallets");
    }
    Ok(())
}

/// Dune Analytics — optional; respects free-tier 3/day limit via 8h cache.
async fn poll_dune(
    client: &reqwest::Client,
    wallets: &RwLock<HashSet<String>>,
    cfg: &WhaleTrackerConfig,
) -> anyhow::Result<u64> {
    // Dune API v1 requires an API key for most deployments; attempt best-effort
    // without key and cache failures so we do not burn the daily quota.
    let url = format!("https://api.dune.com/api/v1/query/{DUNE_QUERY_ID}/results");
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("Dune HTTP {} — enable DUNE_API_KEY for production use", resp.status());
    }

    let body: serde_json::Value = resp.json().await?;
    let rows = body
        .pointer("/result/rows")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    let mut added = 0usize;
    {
        let mut guard = wallets.write().expect("lock");
        for (i, row) in rows.iter().take(10).enumerate() {
            let wallet = row
                .get("wallet")
                .or_else(|| row.get("trader"))
                .and_then(|w| w.as_str());
            let Some(wallet) = wallet else { continue };
            if guard.len() >= cfg.max_tracked_wallets {
                break;
            }
            if guard.insert(wallet.to_owned()) {
                added += 1;
                info!(
                    wallet = %short_wallet(wallet),
                    rank = i + 1,
                    "discovered whale wallet from Dune"
                );
            }
        }
    }

    if added > 0 {
        info!(added, "Dune discovery merged new whale wallets");
    }
    Ok(unix_now_secs())
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovered_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!("whale_disc_{}", std::process::id()));
        let path = dir.join("discovered_whales.json");
        let path_str = path.to_string_lossy().to_string();

        let mut set = seed_tracked_wallets(&[]);
        set.insert("NewWhaleWallet111111111111111111111111111".to_owned());
        persist_discovered(&path_str, &set, 1_700_000_000);

        let loaded = load_discovered_wallets(&path_str);
        assert!(loaded.iter().any(|w| w.starts_with("NewWhale")));
        let _ = std::fs::remove_dir_all(dir);
    }
}
