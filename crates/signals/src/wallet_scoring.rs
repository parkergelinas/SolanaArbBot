//! Wallet scoring for copy-trading qualification.
//!
//! Scores are sourced from GMGN top-trader API (same pattern as
//! [`crate::whale_discovery`]) with optional cached Dune rows.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use config::CopyTradingConfig;
use tracing::{debug, info, warn};

use crate::whale_watcher::{short_wallet, WHALE_WALLETS};

const GMGN_TOP_TRADERS_URL: &str =
    "https://gmgn.ai/api/v1/sol/top_traders?limit=50&orderby=pnl_30d";
const GMGN_WALLET_STAT_URL: &str = "https://gmgn.ai/api/v1/sol/wallet_stat/";

/// Rolling 30-day wallet performance metrics used for copy-trading gates.
#[derive(Clone, Debug, PartialEq)]
pub struct WalletScore {
    pub wallet: String,
    pub win_rate_30d: f64,
    pub realized_pnl_30d: f64,
    pub trade_count_30d: u32,
    pub avg_hold_seconds: u64,
    pub token_diversity: f64,
    pub bot_score: f64,
    pub concentration_risk: f64,
}

/// Hard-coded structural gates (not overridden by config).
const MAX_AVG_HOLD_SECONDS: u64 = 3600;
const MIN_TOKEN_DIVERSITY: f64 = 0.30;
const MAX_BOT_SCORE: f64 = 0.70;
const MAX_CONCENTRATION_RISK: f64 = 0.40;

/// Returns `true` when every scoring criterion passes.
pub fn qualifies_wallet(score: &WalletScore, cfg: &CopyTradingConfig) -> bool {
    score.win_rate_30d > cfg.min_wallet_win_rate
        && score.realized_pnl_30d > cfg.min_wallet_pnl_30d
        && score.trade_count_30d > cfg.min_wallet_trades_30d
        && score.avg_hold_seconds < MAX_AVG_HOLD_SECONDS
        && score.token_diversity > MIN_TOKEN_DIVERSITY
        && score.bot_score <= MAX_BOT_SCORE
        && score.concentration_risk <= MAX_CONCENTRATION_RISK
}

/// Thread-safe set of wallets approved for copy-trading.
#[derive(Clone, Default)]
pub struct QualifiedWalletSet {
    inner: Arc<RwLock<HashSet<String>>>,
}

impl QualifiedWalletSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace(&self, wallets: HashSet<String>) {
        let mut g = self.inner.write().expect("lock");
        *g = wallets;
    }

    pub fn insert(&self, wallet: &str) {
        self.inner.write().expect("lock").insert(wallet.to_owned());
    }

    pub fn contains(&self, wallet: &str) -> bool {
        self.inner.read().expect("lock").contains(wallet)
    }

    pub fn len(&self) -> usize {
        self.inner.read().expect("lock").len()
    }

    /// Seed whales are always trusted; discovered wallets must qualify.
    pub fn is_tracked(&self, wallet: &str) -> bool {
        WHALE_WALLETS.contains(&wallet) || self.contains(wallet)
    }
}

/// Parse a GMGN trader row into [`WalletScore`].
pub fn score_from_gmgn_row(wallet: &str, row: &serde_json::Value) -> WalletScore {
    WalletScore {
        wallet: wallet.to_owned(),
        win_rate_30d: json_f64(row, &["win_rate_30d", "winrate_30d", "win_rate"], 0.0),
        realized_pnl_30d: json_f64(
            row,
            &["pnl_30d", "realized_profit_30d", "realized_pnl_30d"],
            0.0,
        ),
        trade_count_30d: json_u32(row, &["trade_count_30d", "tx_count_30d", "trades_30d"], 0),
        avg_hold_seconds: json_u64(row, &["avg_hold_seconds", "hold_time_avg", "avg_hold_time"], 0),
        token_diversity: json_f64(row, &["token_diversity", "diversity", "unique_token_ratio"], 0.0),
        bot_score: json_f64(row, &["bot_score", "bot_probability", "is_bot_score"], 0.0),
        concentration_risk: json_f64(
            row,
            &["concentration_risk", "concentration", "top_token_pct"],
            0.0,
        ),
    }
}

fn json_f64(row: &serde_json::Value, keys: &[&str], default: f64) -> f64 {
    for key in keys {
        if let Some(v) = row.get(*key) {
            if let Some(f) = v.as_f64() {
                return f;
            }
            if let Some(s) = v.as_str().and_then(|s| s.parse().ok()) {
                return s;
            }
        }
    }
    default
}

fn json_u32(row: &serde_json::Value, keys: &[&str], default: u32) -> u32 {
    for key in keys {
        if let Some(v) = row.get(*key) {
            if let Some(n) = v.as_u64() {
                return n as u32;
            }
            if let Some(s) = v.as_str().and_then(|s| s.parse().ok()) {
                return s;
            }
        }
    }
    default
}

fn json_u64(row: &serde_json::Value, keys: &[&str], default: u64) -> u64 {
    for key in keys {
        if let Some(v) = row.get(*key) {
            if let Some(n) = v.as_u64() {
                return n;
            }
            if let Some(s) = v.as_str().and_then(|s| s.parse().ok()) {
                return s;
            }
        }
    }
    default
}

/// Poll GMGN and refresh the qualified wallet set.
pub fn spawn_wallet_scoring_poller(
    qualified: QualifiedWalletSet,
    cfg: Arc<CopyTradingConfig>,
    interval_s: u64,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            if let Err(e) = refresh_qualified_from_gmgn(&client, &qualified, &cfg).await {
                warn!(error = %e, "wallet scoring poll failed");
            }
            tokio::time::sleep(Duration::from_secs(interval_s)).await;
        }
    });
    debug!(interval_s, "wallet scoring poller started");
}

async fn refresh_qualified_from_gmgn(
    client: &reqwest::Client,
    qualified: &QualifiedWalletSet,
    cfg: &CopyTradingConfig,
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

    let mut next = HashSet::new();
    for item in &items {
        let wallet = item
            .get("wallet")
            .or_else(|| item.get("address"))
            .and_then(|w| w.as_str());
        let Some(wallet) = wallet else { continue };

        let score = score_from_gmgn_row(wallet, item);
        if qualifies_wallet(&score, cfg) {
            next.insert(wallet.to_owned());
            info!(
                wallet = %short_wallet(wallet),
                win_rate = score.win_rate_30d,
                pnl_30d = score.realized_pnl_30d,
                trades = score.trade_count_30d,
                "wallet qualified for copy trading"
            );
        }
    }

    qualified.replace(next);
    info!(qualified = qualified.len(), "wallet scoring refresh complete");
    Ok(())
}

/// Fetch per-wallet stats from GMGN (used for on-demand scoring).
pub async fn fetch_wallet_score(
    client: &reqwest::Client,
    wallet: &str,
) -> anyhow::Result<WalletScore> {
    let url = format!("{GMGN_WALLET_STAT_URL}{wallet}");
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("GMGN wallet stat HTTP {}", resp.status());
    }
    let body: serde_json::Value = resp.json().await?;
    let row = body.pointer("/data").unwrap_or(&body);
    Ok(score_from_gmgn_row(wallet, row))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_score() -> WalletScore {
        WalletScore {
            wallet: "TestWallet".to_owned(),
            win_rate_30d: 0.65,
            realized_pnl_30d: 15_000.0,
            trade_count_30d: 80,
            avg_hold_seconds: 1800,
            token_diversity: 0.45,
            bot_score: 0.20,
            concentration_risk: 0.25,
        }
    }

    #[test]
    fn qualifies_wallet_passes_all_criteria() {
        let cfg = CopyTradingConfig::default();
        assert!(qualifies_wallet(&sample_score(), &cfg));
    }

    #[test]
    fn rejects_low_win_rate() {
        let cfg = CopyTradingConfig::default();
        let mut s = sample_score();
        s.win_rate_30d = 0.55;
        assert!(!qualifies_wallet(&s, &cfg));
    }

    #[test]
    fn rejects_high_bot_score() {
        let cfg = CopyTradingConfig::default();
        let mut s = sample_score();
        s.bot_score = 0.75;
        assert!(!qualifies_wallet(&s, &cfg));
    }

    #[test]
    fn rejects_long_hold_time() {
        let cfg = CopyTradingConfig::default();
        let mut s = sample_score();
        s.avg_hold_seconds = 7200;
        assert!(!qualifies_wallet(&s, &cfg));
    }

    #[test]
    fn rejects_low_pnl() {
        let cfg = CopyTradingConfig::default();
        let mut s = sample_score();
        s.realized_pnl_30d = 5_000.0;
        assert!(!qualifies_wallet(&s, &cfg));
    }

    #[test]
    fn seed_wallets_always_tracked() {
        let q = QualifiedWalletSet::new();
        assert!(q.is_tracked(WHALE_WALLETS[0]));
    }

    #[test]
    fn score_from_gmgn_row_parses_fields() {
        let row = serde_json::json!({
            "win_rate_30d": 0.72,
            "pnl_30d": 25000,
            "trade_count_30d": 120,
            "avg_hold_seconds": 900,
            "token_diversity": 0.5,
            "bot_score": 0.1,
            "concentration_risk": 0.2
        });
        let score = score_from_gmgn_row("Wallet1", &row);
        assert!((score.win_rate_30d - 0.72).abs() < f64::EPSILON);
        assert_eq!(score.trade_count_30d, 120);
    }
}
