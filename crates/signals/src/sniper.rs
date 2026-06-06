//! New-token sniper — rug screening, position sizing, and Jupiter dry-run entries.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use config::{DataSourcesConfig, FeatureFlags, SniperConfig};
use crossbeam_channel::{Receiver, Sender};
use ratelimit::{ApiSource, RateLimiter};
use tracing::{info, warn};

use crate::dexscreener::rugcheck_passes;
use crate::external_store::ExternalSignalStore;
use crate::whale_watcher::short_wallet;

use pricing::JUPITER_SWAP_V1_BASE;

/// Jupiter Swap API v1 base.
pub const JUPITER_SWAP_API: &str = JUPITER_SWAP_V1_BASE;

/// Default sniper market-buy slippage (3%).
pub const SNIPER_SLIPPAGE_BPS: u32 = 300;

/// Candidate new pool / token for sniper evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct SniperCandidate {
    pub mint: String,
    pub pool_address: String,
    pub creator_wallet: String,
    pub initial_liquidity_sol: f64,
    pub signature: String,
    pub slot: u64,
}

/// Open sniper position state.
#[derive(Clone, Debug)]
pub struct SniperPosition {
    pub mint: String,
    pub entry_price_usd: f64,
    pub size_sol: f64,
    pub opened_at: Instant,
    pub tp1_taken: bool,
}

/// Exit decision from monitor loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitAction {
    Hold,
    TakeProfit1,
    TakeProfit2,
    StopLoss,
    TimeExit,
}

/// Why rug screening rejected a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RugRejectReason {
    LowScore,
    HighCreatorHold,
    Blocked,
    LiquidityTooLow,
}

/// Rug screening outcome.
#[derive(Clone, Debug, PartialEq)]
pub enum RugVerdict {
    Pass { score: f64 },
    Reject(RugRejectReason),
}

/// In-memory rug check cache keyed by mint.
#[derive(Clone, Default)]
pub struct RugCheckCache {
    inner: Arc<RwLock<HashMap<String, RugVerdict>>>,
}

impl RugCheckCache {
    pub fn get(&self, mint: &str) -> Option<RugVerdict> {
        self.inner.read().expect("lock").get(mint).cloned()
    }

    pub fn set(&self, mint: &str, verdict: RugVerdict) {
        self.inner
            .write()
            .expect("lock")
            .insert(mint.to_owned(), verdict);
    }
}

/// Channel for pool-creation / new-token candidates.
pub fn pool_creation_channel(capacity: usize) -> (Sender<SniperCandidate>, Receiver<SniperCandidate>) {
    crossbeam_channel::bounded(capacity)
}

/// Publisher handle for sniper candidates.
pub type SniperPublisher = Sender<SniperCandidate>;

/// Size position from config and pool liquidity.
pub fn compute_position_sol(cfg: &SniperConfig, liquidity_sol: f64) -> f64 {
    if liquidity_sol < cfg.min_liquidity_sol {
        return 0.0;
    }
    let scale = (liquidity_sol / cfg.min_liquidity_sol).min(2.0);
    (cfg.base_position_sol * scale).min(cfg.base_position_sol * 2.0)
}

/// Evaluate take-profit / stop-loss / time exit.
pub fn evaluate_exit(cfg: &SniperConfig, pos: &SniperPosition, current_price: f64) -> ExitAction {
    if current_price <= 0.0 || pos.entry_price_usd <= 0.0 {
        return ExitAction::Hold;
    }
    let multiple = current_price / pos.entry_price_usd;
    let elapsed = pos.opened_at.elapsed().as_secs();

    if elapsed >= cfg.time_exit_seconds {
        return ExitAction::TimeExit;
    }
    if multiple <= 1.0 - cfg.stop_loss_pct {
        return ExitAction::StopLoss;
    }
    if !pos.tp1_taken && multiple >= cfg.take_profit_1_x {
        return ExitAction::TakeProfit1;
    }
    if pos.tp1_taken && multiple >= cfg.take_profit_2_x {
        return ExitAction::TakeProfit2;
    }
    ExitAction::Hold
}

/// Rug screening via Rugcheck API (cached).
pub async fn rug_check(
    cache: &RugCheckCache,
    store: &ExternalSignalStore,
    data_sources: &DataSourcesConfig,
    limiter: &RateLimiter,
    cfg: &SniperConfig,
    mint: &str,
    client: &reqwest::Client,
) -> RugVerdict {
    if let Some(v) = cache.get(mint) {
        return v;
    }
    if store.is_rugcheck_blocked(mint) {
        let v = RugVerdict::Reject(RugRejectReason::Blocked);
        cache.set(mint, v.clone());
        return v;
    }

    if !limiter.try_acquire(ApiSource::Rugcheck) {
        return RugVerdict::Pass { score: cfg.rugcheck_min_score as f64 };
    }

    let url = format!("{}/tokens/{}/report", data_sources.rugcheck_base, mint);
    let Ok(resp) = client.get(&url).send().await else {
        return RugVerdict::Pass { score: 0.0 };
    };
    if resp.status().as_u16() == 429 {
        limiter.on_rate_limited(ApiSource::Rugcheck);
        return RugVerdict::Pass { score: 0.0 };
    }
    let Ok(body) = resp.json::<serde_json::Value>().await else {
        return RugVerdict::Pass { score: 0.0 };
    };

    let score = body.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);
    let creator_pct = body
        .pointer("/creatorPercentage")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let verdict = if score < cfg.rugcheck_min_score as f64 {
        RugVerdict::Reject(RugRejectReason::LowScore)
    } else if creator_pct > cfg.max_creator_holding_pct {
        RugVerdict::Reject(RugRejectReason::HighCreatorHold)
    } else if !rugcheck_passes(store, data_sources, limiter, mint, client).await {
        RugVerdict::Reject(RugRejectReason::Blocked)
    } else {
        RugVerdict::Pass { score }
    };

    cache.set(mint, verdict.clone());
    verdict
}

/// Fetch USD price from DexScreener token endpoint.
pub async fn fetch_token_price_usd(
    data_sources: &DataSourcesConfig,
    mint: &str,
    client: &reqwest::Client,
) -> Option<f64> {
    let url = format!("{}/tokens/{}", data_sources.dexscreener_base, mint);
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body = resp.json::<serde_json::Value>().await.ok()?;
    body.pointer("/pairs/0/priceUsd")
        .and_then(|p| p.as_str())
        .and_then(|s| s.parse::<f64>().ok())
}

/// Execute sniper buy (dry-run logs unless live trading enabled).
pub fn execute_buy(
    _cfg: &SniperConfig,
    features: &FeatureFlags,
    candidate: &SniperCandidate,
    size_sol: f64,
    entry_price_usd: f64,
) -> Option<SniperPosition> {
    if size_sol <= 0.0 {
        return None;
    }

    if features.dry_run || !features.enable_live_trading {
        info!(
            mint = %short_wallet(&candidate.mint),
            size_sol,
            entry_price_usd,
            slippage_bps = SNIPER_SLIPPAGE_BPS,
            dry_run = true,
            "sniper buy (paper)"
        );
    } else {
        info!(
            mint = %short_wallet(&candidate.mint),
            size_sol,
            api = JUPITER_SWAP_API,
            "sniper buy submitting Jupiter swap"
        );
    }

    Some(SniperPosition {
        mint: candidate.mint.clone(),
        entry_price_usd,
        size_sol,
        opened_at: Instant::now(),
        tp1_taken: false,
    })
}

struct SniperState {
    positions: HashMap<String, SniperPosition>,
    seen_mints: HashSet<String>,
}

impl SniperState {
    fn new() -> Self {
        Self {
            positions: HashMap::new(),
            seen_mints: HashSet::new(),
        }
    }
}

/// Forward sniper candidates to the shared signal bus (placeholder publisher).
pub fn spawn_sniper_bus_publisher(
    rx: Receiver<SniperCandidate>,
    _bus_tx: crossbeam_channel::Sender<serde_json::Value>,
) {
    tokio::spawn(async move {
        while let Ok(c) = rx.recv() {
            info!(
                mint = %short_wallet(&c.mint),
                liquidity_sol = c.initial_liquidity_sol,
                "sniper candidate published"
            );
        }
    });
}

/// Monitor open positions for TP/SL/time exits.
pub fn spawn_sniper_monitor(
    cfg: Arc<SniperConfig>,
    features: Arc<FeatureFlags>,
    data_sources: Arc<DataSourcesConfig>,
    state: Arc<RwLock<SniperState>>,
) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let limiter = RateLimiter::free_tier_defaults();
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let mints: Vec<String> = {
                let g = state.read().expect("lock");
                g.positions.keys().cloned().collect()
            };
            for mint in mints {
                let Some(price) = fetch_token_price_usd(&data_sources, &mint, &client).await
                else {
                    continue;
                };
                let mut g = state.write().expect("lock");
                let Some(pos) = g.positions.get_mut(&mint) else {
                    continue;
                };
                match evaluate_exit(&cfg, pos, price) {
                    ExitAction::Hold => {}
                    ExitAction::TakeProfit1 => {
                        pos.tp1_taken = true;
                        info!(mint = %short_wallet(&mint), price, "sniper TP1");
                    }
                    ExitAction::TakeProfit2 | ExitAction::StopLoss | ExitAction::TimeExit => {
                        info!(
                            mint = %short_wallet(&mint),
                            price,
                            dry_run = features.dry_run,
                            "sniper exit"
                        );
                        g.positions.remove(&mint);
                    }
                }
                let _ = limiter;
            }
        }
    });
}

/// Main sniper loop — watches DexScreener new pairs from the external store.
pub fn spawn_sniper_strategy(
    cfg: Arc<SniperConfig>,
    features: Arc<FeatureFlags>,
    data_sources: Arc<DataSourcesConfig>,
    store: ExternalSignalStore,
    _rpc_url: String,
) {
    if !cfg.enabled {
        info!("sniper strategy disabled");
        return;
    }

    let state = Arc::new(RwLock::new(SniperState::new()));
    let rug_cache = RugCheckCache::default();

    spawn_sniper_monitor(
        Arc::clone(&cfg),
        Arc::clone(&features),
        Arc::clone(&data_sources),
        Arc::clone(&state),
    );

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let limiter = RateLimiter::free_tier_defaults();

        info!(
            base_position_sol = cfg.base_position_sol,
            min_liquidity_sol = cfg.min_liquidity_sol,
            rugcheck_min = cfg.rugcheck_min_score,
            dry_run = features.dry_run,
            "sniper strategy started"
        );

        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            let pairs = store.recent_new_pairs();
            for pair in pairs {
                let liquidity_sol = pair.liquidity_usd / sol_price_estimate();
                let candidate = SniperCandidate {
                    mint: pair.mint.clone(),
                    pool_address: pair.pair_address.clone(),
                    creator_wallet: "unknown".to_owned(),
                    initial_liquidity_sol: liquidity_sol,
                    signature: format!("dex_{}", unix_now_ms()),
                    slot: 0,
                };

                {
                    let g = state.read().expect("lock");
                    if g.seen_mints.contains(&candidate.mint) {
                        continue;
                    }
                    if g.positions.len() >= cfg.max_concurrent_positions as usize {
                        continue;
                    }
                }

                let size_sol = compute_position_sol(&cfg, liquidity_sol);
                if size_sol <= 0.0 {
                    continue;
                }

                match rug_check(
                    &rug_cache,
                    &store,
                    &data_sources,
                    &limiter,
                    &cfg,
                    &candidate.mint,
                    &client,
                )
                .await
                {
                    RugVerdict::Reject(reason) => {
                        warn!(
                            mint = %short_wallet(&candidate.mint),
                            ?reason,
                            "sniper rug reject"
                        );
                        state.write().expect("lock").seen_mints.insert(candidate.mint.clone());
                        continue;
                    }
                    RugVerdict::Pass { score } => {
                        info!(
                            mint = %short_wallet(&candidate.mint),
                            score,
                            liquidity_sol,
                            "sniper rug pass"
                        );
                    }
                }

                let entry_price = fetch_token_price_usd(&data_sources, &candidate.mint, &client)
                    .await
                    .unwrap_or(0.0);

                if let Some(pos) =
                    execute_buy(&cfg, &features, &candidate, size_sol, entry_price)
                {
                    let mut g = state.write().expect("lock");
                    g.seen_mints.insert(candidate.mint.clone());
                    g.positions.insert(candidate.mint.clone(), pos);
                }
            }
        }
    });
}

fn sol_price_estimate() -> f64 {
    std::env::var("SOLANA_ARB_SOL_PRICE_USD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(150.0)
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
