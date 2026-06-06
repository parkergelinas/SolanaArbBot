//! Liquidation Hunter — scan lending protocol borrow obligations and execute
//! profitable liquidations when health drops below critical threshold.
//!
//! Supports Kamino (full `getProgramAccounts` scan), with MarginFi / Drift /
//! Save stubs that follow the same RPC polling pattern until SDK wiring lands.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use config::{DataSourcesConfig, FeatureFlags, LiquidationConfig};
use ratelimit::{ApiSource, RateLimiter};
use tracing::{debug, info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Protocol program IDs
// ─────────────────────────────────────────────────────────────────────────────

pub const KAMINO_PROGRAM: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const MARGINFI_PROGRAM: &str = "4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG";
pub const DRIFT_PROGRAM: &str = "dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH";
pub const SAVE_PROGRAM: &str = "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo";

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const JUPITER_V6: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

// ─────────────────────────────────────────────────────────────────────────────
// Pyth oracle price feed accounts (mainnet)
// ─────────────────────────────────────────────────────────────────────────────

pub const PYTH_SOL_USD: &str = "H6ARHf6YXhGYeQfUzQGQkuLdLBf3z5M8V5q2tXgQjV8";
pub const PYTH_BTC_USD: &str = "GVnyRldT57LW54tL8k9jF4p2XvLh8qJ8TqN7Q8K1J3V";
pub const PYTH_ETH_USD: &str = "JBu1AL4obBcCMqKBBxhpYoFDzMTr6beM9SU5gAYnghtw";

pub const PYTH_FEEDS: &[(&str, &str)] = &[
    ("SOL", PYTH_SOL_USD),
    ("BTC", PYTH_BTC_USD),
    ("ETH", PYTH_ETH_USD),
];

const PRICE_DROP_TRIGGER_PCT: f64 = 3.0;
const PYTH_POLL_INTERVAL_MS: u64 = 400;

// ─────────────────────────────────────────────────────────────────────────────
// Core types
// ─────────────────────────────────────────────────────────────────────────────

/// Supported lending protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LendingProtocol {
    Kamino,
    MarginFi,
    Drift,
    Save,
}

impl LendingProtocol {
    pub fn program_id(self) -> &'static str {
        match self {
            Self::Kamino => KAMINO_PROGRAM,
            Self::MarginFi => MARGINFI_PROGRAM,
            Self::Drift => DRIFT_PROGRAM,
            Self::Save => SAVE_PROGRAM,
        }
    }

    pub fn from_str_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "kamino" => Some(Self::Kamino),
            "marginfi" | "margin_fi" => Some(Self::MarginFi),
            "drift" => Some(Self::Drift),
            "save" | "solend" => Some(Self::Save),
            _ => None,
        }
    }
}

/// Health classification tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthTier {
    Critical,
    Watch,
    Safe,
}

/// Open borrow position across a lending protocol.
#[derive(Clone, Debug, PartialEq)]
pub struct BorrowPosition {
    pub protocol: LendingProtocol,
    pub obligation_pubkey: String,
    pub owner: String,
    pub collateral_value_usd: f64,
    pub borrow_value_usd: f64,
    /// Liquidation threshold (e.g. 0.85 for 85% LTV at liquidation).
    pub liq_threshold: f64,
    pub health: f64,
    pub tier: HealthTier,
    pub last_scanned: u64,
}

/// Snapshot of a Pyth price feed.
#[derive(Clone, Debug)]
pub struct PythPriceSnapshot {
    pub symbol: String,
    pub feed: String,
    pub price_usd: f64,
    pub slot: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Health math (pure — fully unit-tested)
// ─────────────────────────────────────────────────────────────────────────────

/// Compute health factor: `(collateral * liq_threshold) / borrow`.
///
/// Returns `f64::MAX` when borrow is zero (fully safe, no debt).
#[inline]
pub fn compute_health(collateral_value: f64, borrow_value: f64, liq_threshold: f64) -> f64 {
    if borrow_value <= f64::EPSILON {
        return f64::MAX;
    }
    (collateral_value * liq_threshold) / borrow_value
}

/// Classify a health factor into CRITICAL / WATCH / SAFE tiers.
#[inline]
pub fn classify_health(
    health: f64,
    watch_threshold: f64,
    critical_threshold: f64,
) -> HealthTier {
    if health < critical_threshold {
        HealthTier::Critical
    } else if health < watch_threshold {
        HealthTier::Watch
    } else {
        HealthTier::Safe
    }
}

/// Sort positions by liquidation urgency: `(1.0 - health)` descending.
pub fn sort_positions_by_urgency(positions: &mut [BorrowPosition]) {
    positions.sort_by(|a, b| {
        let urgency_a = 1.0 - a.health.min(2.0);
        let urgency_b = 1.0 - b.health.min(2.0);
        urgency_b
            .partial_cmp(&urgency_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Build a [`BorrowPosition`] with computed health and tier.
pub fn build_position(
    protocol: LendingProtocol,
    obligation_pubkey: String,
    owner: String,
    collateral_value_usd: f64,
    borrow_value_usd: f64,
    liq_threshold: f64,
    watch_threshold: f64,
    critical_threshold: f64,
) -> BorrowPosition {
    let health = compute_health(collateral_value_usd, borrow_value_usd, liq_threshold);
    let tier = classify_health(health, watch_threshold, critical_threshold);
    BorrowPosition {
        protocol,
        obligation_pubkey,
        owner,
        collateral_value_usd,
        borrow_value_usd,
        liq_threshold,
        health,
        tier,
        last_scanned: unix_now_secs(),
    }
}

/// Returns true when a price drop exceeds `drop_pct` between two observations.
pub fn price_drop_exceeds(prev: f64, current: f64, drop_pct: f64) -> bool {
    if prev <= f64::EPSILON {
        return false;
    }
    let change_pct = (current - prev) / prev * 100.0;
    change_pct <= -drop_pct
}

// ─────────────────────────────────────────────────────────────────────────────
// Position store
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct StoreInner {
    positions: HashMap<String, BorrowPosition>,
}

/// Thread-safe cache of tracked borrow positions keyed by obligation pubkey.
#[derive(Clone, Default)]
pub struct LiquidationStore {
    inner: Arc<RwLock<StoreInner>>,
}

impl std::fmt::Debug for LiquidationStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiquidationStore").finish_non_exhaustive()
    }
}

impl LiquidationStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&self, position: BorrowPosition) {
        let key = position.obligation_pubkey.clone();
        self.inner
            .write()
            .expect("lock")
            .positions
            .insert(key, position);
    }

    pub fn all_positions(&self) -> Vec<BorrowPosition> {
        self.inner
            .read()
            .expect("lock")
            .positions
            .values()
            .cloned()
            .collect()
    }

    pub fn watch_and_critical(&self) -> Vec<BorrowPosition> {
        self.all_positions()
            .into_iter()
            .filter(|p| p.tier == HealthTier::Watch || p.tier == HealthTier::Critical)
            .collect()
    }

    pub fn positions_by_tier(&self, tier: HealthTier) -> Vec<BorrowPosition> {
        self.all_positions()
            .into_iter()
            .filter(|p| p.tier == tier)
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Protocol scanners
// ─────────────────────────────────────────────────────────────────────────────

/// Scan all configured protocols and return positions above `min_value_usd`.
pub async fn scan_all_protocols(
    cfg: &LiquidationConfig,
    rpc_url: &str,
    limiter: &RateLimiter,
) -> Vec<BorrowPosition> {
    let mut all = Vec::new();

    for name in &cfg.protocols {
        let Some(protocol) = LendingProtocol::from_str_name(name) else {
            warn!(protocol = %name, "unknown liquidation protocol — skipping");
            continue;
        };

        let batch = match protocol {
            LendingProtocol::Kamino => {
                scan_kamino_obligations(cfg, rpc_url, limiter).await
            }
            LendingProtocol::MarginFi => {
                scan_marginfi_accounts(cfg, rpc_url, limiter).await
            }
            LendingProtocol::Drift => scan_drift_accounts(cfg, rpc_url, limiter).await,
            LendingProtocol::Save => scan_save_obligations(cfg, rpc_url, limiter).await,
        };

        all.extend(batch);
    }

    all.retain(|p| p.borrow_value_usd >= cfg.min_position_value_usd);
    sort_positions_by_urgency(&mut all);
    all
}

/// Kamino: `getProgramAccounts` on KLend program with base64 encoding.
async fn scan_kamino_obligations(
    cfg: &LiquidationConfig,
    rpc_url: &str,
    limiter: &RateLimiter,
) -> Vec<BorrowPosition> {
    if !limiter.try_acquire(ApiSource::Helius) {
        return Vec::new();
    }

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getProgramAccounts",
        "params": [
            KAMINO_PROGRAM,
            {
                "encoding": "base64",
                "filters": [
                    { "dataSize": 916 }
                ]
            }
        ]
    });

    let resp = match client.post(rpc_url).json(&body).send().await {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            debug!(status = %r.status(), "kamino getProgramAccounts failed");
            return Vec::new();
        }
        Err(e) => {
            debug!(error = %e, "kamino RPC error");
            return Vec::new();
        }
    };

    let v: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let accounts = match v.get("result").and_then(|r| r.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };

    let mut positions = Vec::new();
    for acct in accounts {
        let pubkey = acct
            .get("pubkey")
            .and_then(|p| p.as_str())
            .unwrap_or_default()
            .to_owned();

        // Stub: real implementation deserializes Kamino Obligation account layout.
        // For now derive placeholder values from account data length so scanner structure is wired.
        let data_len = acct
            .pointer("/account/data/0")
            .and_then(|d| d.as_str())
            .map(|s| s.len())
            .unwrap_or(0);

        if data_len == 0 {
            continue;
        }

        // Placeholder health inputs — replaced when Kamino layout decoder lands.
        let collateral = 5_000.0 + (data_len as f64 % 500.0);
        let borrow = 4_500.0 + (data_len as f64 % 400.0);
        let liq_threshold = 0.85;

        let pos = build_position(
            LendingProtocol::Kamino,
            pubkey,
            String::new(),
            collateral,
            borrow,
            liq_threshold,
            cfg.watch_health_threshold,
            cfg.critical_health_threshold,
        );
        positions.push(pos);
    }

    debug!(count = positions.len(), "kamino obligations scanned");
    positions
}

/// MarginFi stub — polls `getProgramAccounts` with dataSize filter.
async fn scan_marginfi_accounts(
    cfg: &LiquidationConfig,
    rpc_url: &str,
    limiter: &RateLimiter,
) -> Vec<BorrowPosition> {
    if !limiter.try_acquire(ApiSource::Helius) {
        return Vec::new();
    }

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getProgramAccounts",
        "params": [
            MARGINFI_PROGRAM,
            { "encoding": "base64" }
        ]
    });

    let resp = client.post(rpc_url).json(&body).send().await;
    let Ok(resp) = resp else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }

    let v: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let count = v
        .get("result")
        .and_then(|r| r.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    debug!(count, "marginfi accounts fetched (stub parser)");
    let _ = cfg;
    Vec::new()
}

/// Drift stub — polls program accounts; full user account decode pending SDK.
async fn scan_drift_accounts(
    cfg: &LiquidationConfig,
    rpc_url: &str,
    limiter: &RateLimiter,
) -> Vec<BorrowPosition> {
    if !limiter.try_acquire(ApiSource::Helius) {
        return Vec::new();
    }

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getProgramAccounts",
        "params": [
            DRIFT_PROGRAM,
            { "encoding": "base64", "dataSlice": { "offset": 0, "length": 8 } }
        ]
    });

    let _ = client.post(rpc_url).json(&body).send().await;
    debug!("drift scan stub — awaiting SDK integration");
    let _ = cfg;
    Vec::new()
}

/// Save/Solend stub — same RPC pattern as Kamino obligations.
async fn scan_save_obligations(
    cfg: &LiquidationConfig,
    rpc_url: &str,
    limiter: &RateLimiter,
) -> Vec<BorrowPosition> {
    if !limiter.try_acquire(ApiSource::Helius) {
        return Vec::new();
    }

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getProgramAccounts",
        "params": [
            SAVE_PROGRAM,
            { "encoding": "base64" }
        ]
    });

    let _ = client.post(rpc_url).json(&body).send().await;
    debug!("save/solend scan stub — awaiting obligation layout decoder");
    let _ = cfg;
    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────────────
// Pyth oracle watcher
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct PythFeedState {
    last_price: f64,
    last_slot: u64,
}

type PythStateMap = Arc<RwLock<HashMap<String, PythFeedState>>>;

/// Fetch current Pyth prices via `getAccountInfo` (fallback when gRPC unavailable).
async fn fetch_pyth_prices(rpc_url: &str, limiter: &RateLimiter) -> Vec<PythPriceSnapshot> {
    let client = reqwest::Client::new();
    let mut snapshots = Vec::new();

    for (symbol, feed) in PYTH_FEEDS {
        if !limiter.try_acquire(ApiSource::Helius) {
            break;
        }

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [feed, { "encoding": "base64" }]
        });

        let resp = match client.post(rpc_url).json(&body).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let v: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        let slot = v
            .pointer("/result/context/slot")
            .and_then(|s| s.as_u64())
            .unwrap_or(0);

        // Stub price extraction — real impl parses Pyth PriceAccount binary layout.
        let data_b64 = v
            .pointer("/result/value/data/0")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        let price = parse_pyth_price_stub(data_b64, symbol);

        snapshots.push(PythPriceSnapshot {
            symbol: (*symbol).to_owned(),
            feed: (*feed).to_owned(),
            price_usd: price,
            slot,
        });
    }

    snapshots
}

/// Stub Pyth price parser — returns a deterministic placeholder from account data hash.
fn parse_pyth_price_stub(data_b64: &str, symbol: &str) -> f64 {
    let base = match symbol {
        "SOL" => 150.0,
        "BTC" => 95_000.0,
        "ETH" => 3_500.0,
        _ => 1.0,
    };
    if data_b64.is_empty() {
        return base;
    }
    let perturb = (data_b64.len() % 100) as f64 / 10_000.0;
    base * (1.0 - perturb)
}

/// Check Pyth snapshots against prior state; returns symbols that dropped > threshold.
fn detect_price_drops(
    state: &PythStateMap,
    snapshots: &[PythPriceSnapshot],
    drop_pct: f64,
) -> Vec<String> {
    let mut dropped = Vec::new();
    let mut guard = state.write().expect("lock");

    for snap in snapshots {
        if let Some(prev) = guard.get(&snap.feed) {
            if price_drop_exceeds(prev.last_price, snap.price_usd, drop_pct) {
                dropped.push(snap.symbol.clone());
                info!(
                    symbol = %snap.symbol,
                    prev = prev.last_price,
                    current = snap.price_usd,
                    slot = snap.slot,
                    "Pyth price drop > {drop_pct}% — re-scoring WATCH positions"
                );
            }
        }
        guard.insert(
            snap.feed.clone(),
            PythFeedState {
                last_price: snap.price_usd,
                last_slot: snap.slot,
            },
        );
    }

    dropped
}

// ─────────────────────────────────────────────────────────────────────────────
// Liquidation execution (stub ix builder — real bundle via Jito pattern)
// ─────────────────────────────────────────────────────────────────────────────

/// Planned liquidation action.
#[derive(Clone, Debug)]
pub struct LiquidationPlan {
    pub position: BorrowPosition,
    pub repay_amount_usd: f64,
    pub expected_bonus_usd: f64,
    pub jito_tip_lamports: u64,
    pub needs_jupiter_swap: bool,
}

/// Build a liquidation plan respecting config limits.
pub fn plan_liquidation(position: &BorrowPosition, cfg: &LiquidationConfig) -> LiquidationPlan {
    let max_repay = position.borrow_value_usd * cfg.max_repay_pct;
    let expected_bonus = max_repay * 0.05;
    let tip_sol = expected_bonus * cfg.jito_tip_pct_of_bonus / 150.0;
    let jito_tip_lamports = (tip_sol * 1_000_000_000.0) as u64;

    LiquidationPlan {
        position: position.clone(),
        repay_amount_usd: max_repay,
        expected_bonus_usd: expected_bonus,
        jito_tip_lamports,
        needs_jupiter_swap: true,
    }
}

/// Execute liquidation: Jupiter swap (if needed) + liquidation ix in Jito bundle.
///
/// When `dry_run` is true, logs the plan without submitting transactions.
pub async fn execute_liquidation(
    plan: &LiquidationPlan,
    cfg: &LiquidationConfig,
    features: &FeatureFlags,
    rpc_url: Option<&str>,
) {
    let live = features.enable_live_trading && !features.dry_run;

    info!(
        protocol = ?plan.position.protocol,
        obligation = %short_key(&plan.position.obligation_pubkey),
        health = plan.position.health,
        repay_usd = plan.repay_amount_usd,
        bonus_usd = plan.expected_bonus_usd,
        tip_lamports = plan.jito_tip_lamports,
        dry_run = !live,
        "liquidation plan"
    );

    if !live {
        debug!("dry_run — skipping on-chain liquidation submission");
        return;
    }

    submit_liquidation_bundle(plan, cfg, rpc_url).await;
}

/// Submit liquidation as a Jito bundle (swap + liquidate + collateral→SOL swap).
async fn submit_liquidation_bundle(
    plan: &LiquidationPlan,
    cfg: &LiquidationConfig,
    rpc_url: Option<&str>,
) {
    // Stub: mirrors execution::hotpath::ColdPathExecutor::submit_jito pattern.
    // Real impl builds:
    //   1. Jupiter swap tx (debt token → repay asset if needed)
    //   2. Protocol liquidation ix (max repay = cfg.max_repay_pct)
    //   3. Jupiter swap tx (seized collateral → SOL)
    //   4. Jito bundle with tip = cfg.jito_tip_pct_of_bonus * expected_bonus

    let _ = rpc_url;
    info!(
        protocol = ?plan.position.protocol,
        tip_lamports = plan.jito_tip_lamports,
        max_repay_pct = cfg.max_repay_pct,
        jito_tip_pct = cfg.jito_tip_pct_of_bonus,
        jupiter = JUPITER_V6,
        "Jito liquidation bundle submitted (stub)"
    );
}

fn short_key(key: &str) -> String {
    let k = key.trim();
    if k.len() <= 10 {
        return "****".to_owned();
    }
    format!("{}..{}", &k[..4], &k[k.len() - 4..])
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Spawner
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn the liquidation hunter background loop.
///
/// No-op when `cfg.enabled` is false.
pub fn spawn_liquidation_hunter(
    cfg: Arc<LiquidationConfig>,
    features: Arc<FeatureFlags>,
    data_sources: Arc<DataSourcesConfig>,
    limiter: Arc<RateLimiter>,
    store: LiquidationStore,
) {
    if !cfg.enabled {
        info!("liquidation hunter disabled in config");
        return;
    }

    let rpc_url = match data_sources.helius_rpc_url() {
        Some(url) => url,
        None => {
            warn!("liquidation hunter skipped — no helius_api_key configured");
            return;
        }
    };

    info!(
        scan_interval_ms = cfg.scan_interval_ms,
        safe_poll_interval_s = cfg.safe_poll_interval_s,
        protocols = ?cfg.protocols,
        dry_run = features.dry_run,
        "liquidation hunter starting"
    );

    let pyth_state: PythStateMap = Arc::new(RwLock::new(HashMap::new()));
    let last_safe_poll: Arc<RwLock<Instant>> = Arc::new(RwLock::new(Instant::now()));

    tokio::spawn(async move {
        let scan_interval = Duration::from_millis(cfg.scan_interval_ms);
        let safe_interval = Duration::from_secs(cfg.safe_poll_interval_s);
        let pyth_interval = Duration::from_millis(PYTH_POLL_INTERVAL_MS);

        let mut last_pyth = Instant::now();

        loop {
            let now = Instant::now();

            // ── Pyth oracle poll ─────────────────────────────────────────────
            if now.duration_since(last_pyth) >= pyth_interval {
                last_pyth = now;
                let snapshots = fetch_pyth_prices(&rpc_url, &limiter).await;
                let dropped = detect_price_drops(&pyth_state, &snapshots, PRICE_DROP_TRIGGER_PCT);

                if !dropped.is_empty() {
                    let watch_positions = store.watch_and_critical();
                    for pos in &watch_positions {
                        if pos.tier == HealthTier::Critical {
                            let plan = plan_liquidation(pos, &cfg);
                            execute_liquidation(&plan, &cfg, &features, Some(&rpc_url)).await;
                        }
                    }
                }
            }

            // ── Tiered position scan ─────────────────────────────────────────
            let safe_due = {
                last_safe_poll
                    .read()
                    .expect("lock")
                    .elapsed()
                    >= safe_interval
            };

            let positions = scan_all_protocols(&cfg, &rpc_url, &limiter).await;

            for pos in positions {
                let should_track = match pos.tier {
                    HealthTier::Critical | HealthTier::Watch => true,
                    HealthTier::Safe => safe_due,
                };

                if !should_track {
                    continue;
                }

                if pos.tier == HealthTier::Critical {
                    info!(
                        protocol = ?pos.protocol,
                        obligation = %short_key(&pos.obligation_pubkey),
                        health = pos.health,
                        borrow_usd = pos.borrow_value_usd,
                        "CRITICAL liquidation candidate"
                    );
                    let plan = plan_liquidation(&pos, &cfg);
                    execute_liquidation(&plan, &cfg, &features, Some(&rpc_url)).await;
                } else if pos.tier == HealthTier::Watch {
                    debug!(
                        protocol = ?pos.protocol,
                        health = pos.health,
                        "WATCH position tracked"
                    );
                }

                store.upsert(pos);
            }

            if safe_due {
                *last_safe_poll.write().expect("lock") = Instant::now();
            }

            tokio::time::sleep(scan_interval).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_health_standard_case() {
        // collateral=1000, borrow=800, threshold=0.85 → health = 850/800 = 1.0625
        let h = compute_health(1000.0, 800.0, 0.85);
        assert!((h - 1.0625).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_health_critical() {
        // collateral=1000, borrow=900, threshold=0.85 → health = 850/900 ≈ 0.944
        let h = compute_health(1000.0, 900.0, 0.85);
        assert!(h < 1.0);
    }

    #[test]
    fn compute_health_zero_borrow_is_safe() {
        assert_eq!(compute_health(1000.0, 0.0, 0.85), f64::MAX);
    }

    #[test]
    fn classify_health_tiers() {
        assert_eq!(
            classify_health(0.95, 1.10, 1.00),
            HealthTier::Critical
        );
        assert_eq!(classify_health(1.05, 1.10, 1.00), HealthTier::Watch);
        assert_eq!(classify_health(1.20, 1.10, 1.00), HealthTier::Safe);
    }

    #[test]
    fn classify_health_boundary_values() {
        assert_eq!(classify_health(1.00, 1.10, 1.00), HealthTier::Watch);
        assert_eq!(classify_health(0.999, 1.10, 1.00), HealthTier::Critical);
        assert_eq!(classify_health(1.10, 1.10, 1.00), HealthTier::Safe);
    }

    #[test]
    fn sort_positions_by_urgency_most_critical_first() {
        let mut positions = vec![
            build_position(
                LendingProtocol::Kamino,
                "a".to_owned(),
                String::new(),
                1000.0,
                800.0,
                0.85,
                1.10,
                1.00,
            ),
            build_position(
                LendingProtocol::Kamino,
                "b".to_owned(),
                String::new(),
                1000.0,
                950.0,
                0.85,
                1.10,
                1.00,
            ),
            build_position(
                LendingProtocol::Kamino,
                "c".to_owned(),
                String::new(),
                1000.0,
                850.0,
                0.85,
                1.10,
                1.00,
            ),
        ];

        sort_positions_by_urgency(&mut positions);

        // b has lowest health (~0.895), then c (~0.941), then a (1.0625)
        assert_eq!(positions[0].obligation_pubkey, "b");
        assert_eq!(positions[2].obligation_pubkey, "a");
    }

    #[test]
    fn price_drop_detection() {
        assert!(price_drop_exceeds(100.0, 96.0, 3.0));
        assert!(!price_drop_exceeds(100.0, 98.0, 3.0));
        assert!(!price_drop_exceeds(0.0, 50.0, 3.0));
    }

    #[test]
    fn plan_liquidation_respects_max_repay_pct() {
        let cfg = LiquidationConfig::default();
        let pos = build_position(
            LendingProtocol::Kamino,
            "x".to_owned(),
            String::new(),
            10_000.0,
            8_000.0,
            0.85,
            1.10,
            1.00,
        );
        let plan = plan_liquidation(&pos, &cfg);
        assert!((plan.repay_amount_usd - 4_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lending_protocol_from_name() {
        assert_eq!(
            LendingProtocol::from_str_name("kamino"),
            Some(LendingProtocol::Kamino)
        );
        assert_eq!(
            LendingProtocol::from_str_name("save"),
            Some(LendingProtocol::Save)
        );
        assert_eq!(LendingProtocol::from_str_name("unknown"), None);
    }

    #[test]
    fn store_tier_filtering() {
        let store = LiquidationStore::new();
        store.upsert(build_position(
            LendingProtocol::Kamino,
            "crit".to_owned(),
            String::new(),
            1000.0,
            950.0,
            0.85,
            1.10,
            1.00,
        ));
        store.upsert(build_position(
            LendingProtocol::Kamino,
            "safe".to_owned(),
            String::new(),
            1000.0,
            500.0,
            0.85,
            1.10,
            1.00,
        ));

        assert_eq!(store.positions_by_tier(HealthTier::Critical).len(), 1);
        assert_eq!(store.positions_by_tier(HealthTier::Safe).len(), 1);
        assert_eq!(store.watch_and_critical().len(), 1);
    }
}
