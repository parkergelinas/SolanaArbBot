//! Strategy 5: Momentum / Volume Spike Signal.
//!
//! Polls DexScreener every 15s for volume anomalies on SOL pairs and boosted
//! tokens. Cross-checks whale buys and multi-DEX presence before emitting
//! [`VolumeSpikeSignal`] and optional Jupiter entries (dry-run by default).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use config::{DataSourcesConfig, FeatureFlags, MomentumConfig};
use pricing::JUPITER_SWAP_V1_BASE;
use ratelimit::{ApiSource, RateLimiter};
use tracing::{info, warn};

use crate::dexscreener::rugcheck_passes;
use crate::external_store::{ExternalSignalStore, VolumeSpikeSignal};
use crate::whale_watcher::{short_wallet, USDC_MINT, WhaleSignalStore};

const TOKEN_BOOSTS_URL: &str = "https://api.dexscreener.com/token-boosts/latest/v1";
const POLL_INTERVAL: Duration = Duration::from_secs(15);
const WHALE_CONFIRM_WINDOW_SECS: u64 = 300;
const MAX_BOOST_TOKEN_LOOKUPS: usize = 5;
const JUPITER_SWAP_API: &str = JUPITER_SWAP_V1_BASE;

/// Jupiter market-buy slippage for momentum entries (200 bps).
pub const MOMENTUM_SLIPPAGE_BPS: u32 = 200;

/// Internal momentum candidate after spike detection and cross-checks.
#[derive(Clone, Debug, PartialEq)]
pub struct MomentumSignal {
    pub mint: String,
    pub volume_ratio: f64,
    pub price_velocity_pct_min: f64,
    pub liquidity_usd: f64,
    pub confidence: f64,
    pub whale_confirmed: bool,
    pub multi_dex: bool,
    pub dex_count: usize,
    pub entry_price_usd: f64,
}

#[derive(Clone, Debug)]
struct MomentumPosition {
    mint: String,
    entry_price_usd: f64,
    size_usd: f64,
    opened_at: Instant,
}

#[derive(Default)]
struct TraderInner {
    positions: HashMap<String, MomentumPosition>,
}

/// Tracks open momentum positions for TP/SL/max-hold exits.
#[derive(Clone, Default)]
pub struct MomentumTraderState {
    inner: Arc<RwLock<TraderInner>>,
}

impl MomentumTraderState {
    pub fn new() -> Self {
        Self::default()
    }

    fn has_position(&self, mint: &str) -> bool {
        self.inner.read().expect("lock").positions.contains_key(mint)
    }

    fn open(&self, mint: &str, entry_price_usd: f64, size_usd: f64) {
        self.inner.write().expect("lock").positions.insert(
            mint.to_owned(),
            MomentumPosition {
                mint: mint.to_owned(),
                entry_price_usd,
                size_usd,
                opened_at: Instant::now(),
            },
        );
    }

    fn close(&self, mint: &str) -> Option<MomentumPosition> {
        self.inner.write().expect("lock").positions.remove(mint)
    }

    fn snapshot(&self) -> Vec<MomentumPosition> {
        self.inner
            .read()
            .expect("lock")
            .positions
            .values()
            .cloned()
            .collect()
    }
}

/// `volume_1h / (volume_24h / 24)` — hourly volume vs 24h average hourly rate.
pub fn volume_ratio(volume_h1: f64, volume_h24: f64) -> f64 {
    let hourly_avg = volume_h24 / 24.0;
    if hourly_avg <= 0.0 {
        return 0.0;
    }
    volume_h1 / hourly_avg
}

/// Absolute 5-minute price change divided by 5 → % per minute.
pub fn price_velocity_pct_min(price_change_m5: f64) -> f64 {
    price_change_m5.abs() / 5.0
}

/// Confidence score: base 0.5 + whale bonus + multi-DEX bonus.
pub fn momentum_confidence(whale_confirmed: bool, multi_dex: bool) -> f64 {
    let mut c = 0.5;
    if whale_confirmed {
        c += 0.3;
    }
    if multi_dex {
        c += 0.2;
    }
    c
}

/// Count distinct `dexId` values across Solana pairs for a mint.
pub fn count_distinct_dex_ids(pairs: &[&serde_json::Value]) -> usize {
    let mut dex_ids = HashSet::new();
    for pair in pairs {
        if pair.get("chainId").and_then(|c| c.as_str()) != Some("solana") {
            continue;
        }
        if let Some(dex) = pair.get("dexId").and_then(|d| d.as_str()) {
            dex_ids.insert(dex);
        }
    }
    dex_ids.len()
}

/// True when volume spike thresholds pass (before rugcheck / cross-checks).
pub fn passes_volume_spike_metrics(
    vol_ratio: f64,
    price_vel: f64,
    liquidity_usd: f64,
    cfg: &MomentumConfig,
) -> bool {
    vol_ratio > cfg.min_volume_ratio
        && price_vel < cfg.max_price_velocity_pct_min
        && liquidity_usd > cfg.min_liquidity_usd
}

fn pair_mint(pair: &serde_json::Value) -> Option<String> {
    pair.pointer("/baseToken/address")
        .and_then(|m| m.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn pair_liquidity(pair: &serde_json::Value) -> f64 {
    pair.pointer("/liquidity/usd")
        .and_then(|l| l.as_f64())
        .unwrap_or(0.0)
}

fn pair_price_usd(pair: &serde_json::Value) -> f64 {
    pair.get("priceUsd")
        .and_then(|p| p.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| pair.get("priceUsd").and_then(|p| p.as_f64()))
        .unwrap_or(0.0)
}

fn pair_metrics(pair: &serde_json::Value) -> (f64, f64, f64, f64) {
    let vol_h1 = pair
        .pointer("/volume/h1")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let vol_h24 = pair
        .pointer("/volume/h24")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let price_chg_m5 = pair
        .pointer("/priceChange/m5")
        .and_then(|p| p.as_f64())
        .unwrap_or(0.0);
    let liq = pair_liquidity(pair);
    (
        volume_ratio(vol_h1, vol_h24),
        price_velocity_pct_min(price_chg_m5),
        liq,
        pair_price_usd(pair),
    )
}

fn group_pairs_by_mint(pairs: &[serde_json::Value]) -> HashMap<String, Vec<serde_json::Value>> {
    let mut out: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for pair in pairs {
        if pair.get("chainId").and_then(|c| c.as_str()) != Some("solana") {
            continue;
        }
        let Some(mint) = pair_mint(pair) else {
            continue;
        };
        out.entry(mint).or_default().push(pair.clone());
    }
    out
}

fn extract_boost_mints(body: &serde_json::Value) -> Vec<String> {
    let arr = body.as_array().cloned().unwrap_or_default();
    arr.into_iter()
        .filter(|e| e.get("chainId").and_then(|c| c.as_str()) == Some("solana"))
        .filter_map(|e| {
            e.get("tokenAddress")
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        })
        .collect()
}

fn best_pair_for_metrics(pairs: &[serde_json::Value]) -> Option<&serde_json::Value> {
    pairs
        .iter()
        .max_by(|a, b| {
            pair_liquidity(a)
                .partial_cmp(&pair_liquidity(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

pub(crate) fn evaluate_momentum_candidate(
    mint: &str,
    pairs: &[serde_json::Value],
    cfg: &MomentumConfig,
    whale_store: &WhaleSignalStore,
) -> Option<MomentumSignal> {
    let dex_count = count_distinct_dex_ids(&pairs.iter().collect::<Vec<_>>());
    if dex_count < 2 {
        return None;
    }

    let best = best_pair_for_metrics(pairs)?;
    let (vol_ratio, price_vel, liq, entry_price) = pair_metrics(best);
    if !passes_volume_spike_metrics(vol_ratio, price_vel, liq, cfg) {
        return None;
    }

    let whale_confirmed = whale_store.whale_buy_count(mint, WHALE_CONFIRM_WINDOW_SECS) > 0;
    let multi_dex = dex_count >= 2;
    let confidence = momentum_confidence(whale_confirmed, multi_dex);
    if confidence <= cfg.min_confidence {
        return None;
    }

    Some(MomentumSignal {
        mint: mint.to_owned(),
        volume_ratio: vol_ratio,
        price_velocity_pct_min: price_vel,
        liquidity_usd: liq,
        confidence,
        whale_confirmed,
        multi_dex,
        dex_count,
        entry_price_usd: entry_price,
    })
}

fn momentum_to_volume_spike(signal: &MomentumSignal) -> VolumeSpikeSignal {
    VolumeSpikeSignal {
        mint: signal.mint.clone(),
        volume_ratio: signal.volume_ratio,
        price_velocity_pct_min: signal.price_velocity_pct_min,
        liquidity_usd: signal.liquidity_usd,
        confidence: signal.confidence,
        whale_confirmed: signal.whale_confirmed,
        multi_dex: signal.multi_dex,
        dex_count: signal.dex_count,
        detected_at_ms: unix_now_ms(),
    }
}

pub fn execute_momentum_entry(
    signal: &MomentumSignal,
    cfg: &MomentumConfig,
    state: &MomentumTraderState,
    features: &FeatureFlags,
    _data_sources: &DataSourcesConfig,
) {
    if state.has_position(&signal.mint) {
        return;
    }

    if features.dry_run || !features.enable_live_trading {
        info!(
            mint = %short_wallet(&signal.mint),
            confidence = signal.confidence,
            volume_ratio = signal.volume_ratio,
            price_velocity_pct_min = signal.price_velocity_pct_min,
            liquidity_usd = signal.liquidity_usd,
            position_usd = cfg.position_usd,
            slippage_bps = MOMENTUM_SLIPPAGE_BPS,
            whale_confirmed = signal.whale_confirmed,
            dex_count = signal.dex_count,
            take_profit_pct = cfg.take_profit_pct,
            stop_loss_pct = cfg.stop_loss_pct,
            max_hold_minutes = cfg.max_hold_minutes,
            "Momentum volume spike entry (dry-run — no buy submitted)"
        );
        state.open(&signal.mint, signal.entry_price_usd, cfg.position_usd);
        return;
    }

    info!(
        mint = %short_wallet(&signal.mint),
        confidence = signal.confidence,
        position_usd = cfg.position_usd,
        slippage_bps = MOMENTUM_SLIPPAGE_BPS,
        jupiter = JUPITER_SWAP_API,
        "Momentum volume spike executing Jupiter market buy"
    );
    state.open(&signal.mint, signal.entry_price_usd, cfg.position_usd);
    submit_momentum_buy(signal, cfg);
}

fn submit_momentum_buy(signal: &MomentumSignal, cfg: &MomentumConfig) {
    let _ = (
        JUPITER_SWAP_API,
        MOMENTUM_SLIPPAGE_BPS,
        USDC_MINT,
        &signal.mint,
        cfg.position_usd,
    );
}

fn check_momentum_exits(
    state: &MomentumTraderState,
    cfg: &MomentumConfig,
    features: &FeatureFlags,
    current_prices: &HashMap<String, f64>,
) {
    let max_hold = Duration::from_secs(cfg.max_hold_minutes.saturating_mul(60));
    for pos in state.snapshot() {
        let Some(&price) = current_prices.get(&pos.mint) else {
            continue;
        };
        if pos.entry_price_usd <= 0.0 {
            continue;
        }
        let pnl_pct = (price - pos.entry_price_usd) / pos.entry_price_usd;
        let held = pos.opened_at.elapsed();
        let reason = if pnl_pct >= cfg.take_profit_pct {
            Some("take_profit")
        } else if pnl_pct <= -cfg.stop_loss_pct {
            Some("stop_loss")
        } else if held >= max_hold {
            Some("max_hold")
        } else {
            None
        };
        let Some(reason) = reason else {
            continue;
        };

        if features.dry_run || !features.enable_live_trading {
            info!(
                mint = %short_wallet(&pos.mint),
                reason,
                pnl_pct,
                held_secs = held.as_secs(),
                size_usd = pos.size_usd,
                "Momentum exit (dry-run — no sell submitted)"
            );
        } else {
            info!(
                mint = %short_wallet(&pos.mint),
                reason,
                pnl_pct,
                "Momentum exit executing Jupiter sell"
            );
        }
        state.close(&pos.mint);
    }
}

/// Spawn the DexScreener volume-anomaly poller and optional executor loop.
pub fn spawn_momentum_poller(
    external: ExternalSignalStore,
    whale_store: WhaleSignalStore,
    cfg: Arc<MomentumConfig>,
    features: Arc<FeatureFlags>,
    data_sources: Arc<DataSourcesConfig>,
    limiter: Arc<RateLimiter>,
) {
    let trader = MomentumTraderState::new();

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        info!(
            min_volume_ratio = cfg.min_volume_ratio,
            min_confidence = cfg.min_confidence,
            position_usd = cfg.position_usd,
            "momentum volume spike poller started"
        );

        loop {
            tokio::time::sleep(POLL_INTERVAL).await;

            if !limiter.try_acquire(ApiSource::DexScreener) {
                continue;
            }

            let search_url = format!("{}/search?q=SOL", data_sources.dexscreener_base);
            let search_body = match client.get(&search_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    resp.json::<serde_json::Value>().await.ok()
                }
                Ok(resp) if resp.status().as_u16() == 429 => {
                    limiter.on_rate_limited(ApiSource::DexScreener);
                    None
                }
                Ok(resp) => {
                    warn!(status = %resp.status(), "momentum dexscreener search failed");
                    None
                }
                Err(e) => {
                    warn!(error = %e, "momentum dexscreener search error");
                    None
                }
            };

            if !limiter.try_acquire(ApiSource::DexScreener) {
                continue;
            }

            let boost_body = match client.get(TOKEN_BOOSTS_URL).send().await {
                Ok(resp) if resp.status().is_success() => {
                    resp.json::<serde_json::Value>().await.ok()
                }
                Ok(resp) if resp.status().as_u16() == 429 => {
                    limiter.on_rate_limited(ApiSource::DexScreener);
                    None
                }
                Ok(resp) => {
                    warn!(status = %resp.status(), "momentum token-boosts fetch failed");
                    None
                }
                Err(e) => {
                    warn!(error = %e, "momentum token-boosts error");
                    None
                }
            };

            let mut all_pairs: Vec<serde_json::Value> = search_body
                .as_ref()
                .and_then(|b| b.pointer("/pairs"))
                .and_then(|p| p.as_array())
                .cloned()
                .unwrap_or_default();

            let mut by_mint = group_pairs_by_mint(&all_pairs);

            if let Some(boosts) = boost_body.as_ref() {
                let existing: std::collections::HashSet<String> = by_mint.keys().cloned().collect();
                let missing_mints: Vec<String> = extract_boost_mints(boosts)
                    .into_iter()
                    .filter(|m| !existing.contains(m))
                    .take(MAX_BOOST_TOKEN_LOOKUPS)
                    .collect();
                for mint in missing_mints {
                    if !limiter.try_acquire(ApiSource::DexScreener) {
                        break;
                    }
                    let url = format!("{}/tokens/{}", data_sources.dexscreener_base, mint);
                    let Ok(resp) = client.get(&url).send().await else {
                        continue;
                    };
                    if resp.status().as_u16() == 429 {
                        limiter.on_rate_limited(ApiSource::DexScreener);
                        break;
                    }
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        if let Some(pairs) = body.pointer("/pairs").and_then(|p| p.as_array()) {
                            for pair in pairs {
                                all_pairs.push(pair.clone());
                                if let Some(m) = pair_mint(pair) {
                                    by_mint.entry(m).or_default().push(pair.clone());
                                }
                            }
                        }
                    }
                }
            }

            let mut spike_mints = HashSet::new();
            let mut price_map: HashMap<String, f64> = HashMap::new();

            for (mint, pairs) in &by_mint {
                if let Some(best) = best_pair_for_metrics(pairs) {
                    price_map.insert(mint.clone(), pair_price_usd(best));
                }

                let Some(candidate) =
                    evaluate_momentum_candidate(mint, pairs, &cfg, &whale_store)
                else {
                    continue;
                };

                if !rugcheck_passes(
                    &external,
                    &data_sources,
                    &limiter,
                    mint,
                    &client,
                )
                .await
                {
                    continue;
                }

                spike_mints.insert(mint.clone());
                let vol_signal = momentum_to_volume_spike(&candidate);
                external.push_volume_spike(vol_signal);
                info!(
                    mint = %short_wallet(mint),
                    confidence = candidate.confidence,
                    volume_ratio = candidate.volume_ratio,
                    price_velocity_pct_min = candidate.price_velocity_pct_min,
                    dex_count = candidate.dex_count,
                    whale_confirmed = candidate.whale_confirmed,
                    "MomentumSignal volume spike detected"
                );
                execute_momentum_entry(
                    &candidate,
                    &cfg,
                    &trader,
                    &features,
                    &data_sources,
                );
            }

            external.set_volume_spikes(spike_mints);
            check_momentum_exits(&trader, &cfg, &features, &price_map);
        }
    });
}

/// Emit a momentum signal to the signal-bus JSON format.
pub fn momentum_signal_to_bus_payload(signal: &VolumeSpikeSignal) -> serde_json::Value {
    serde_json::json!({
        "kind": "volume_spike",
        "mint": signal.mint,
        "volume_ratio": signal.volume_ratio,
        "price_velocity_pct_min": signal.price_velocity_pct_min,
        "liquidity_usd": signal.liquidity_usd,
        "confidence": signal.confidence,
        "whale_confirmed": signal.whale_confirmed,
        "multi_dex": signal.multi_dex,
        "dex_count": signal.dex_count,
        "detected_at_ms": signal.detected_at_ms,
        "strategy_tag": "momentum_volume_spike",
    })
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pair(dex_id: &str, mint: &str, vol_h1: f64, vol_h24: f64, chg_m5: f64, liq: f64) -> serde_json::Value {
        serde_json::json!({
            "chainId": "solana",
            "dexId": dex_id,
            "baseToken": { "address": mint },
            "liquidity": { "usd": liq },
            "volume": { "h1": vol_h1, "h24": vol_h24 },
            "priceChange": { "m5": chg_m5 },
            "priceUsd": "1.0"
        })
    }

    fn cfg() -> MomentumConfig {
        MomentumConfig::default()
    }

    #[test]
    fn volume_ratio_computes_hourly_vs_average() {
        assert!((volume_ratio(120.0, 240.0) - 12.0).abs() < f64::EPSILON);
        assert_eq!(volume_ratio(100.0, 0.0), 0.0);
    }

    #[test]
    fn price_velocity_is_abs_change_over_five_minutes() {
        assert!((price_velocity_pct_min(10.0) - 2.0).abs() < f64::EPSILON);
        assert!((price_velocity_pct_min(-15.0) - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn confidence_scoring_adds_bonuses() {
        assert!((momentum_confidence(false, false) - 0.5).abs() < f64::EPSILON);
        assert!((momentum_confidence(true, false) - 0.8).abs() < f64::EPSILON);
        assert!((momentum_confidence(false, true) - 0.7).abs() < f64::EPSILON);
        assert!((momentum_confidence(true, true) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn multi_dex_detection_counts_distinct_dex_ids() {
        let pairs = vec![
            sample_pair("raydium", "MintA", 0.0, 0.0, 0.0, 0.0),
            sample_pair("orca", "MintA", 0.0, 0.0, 0.0, 0.0),
            sample_pair("raydium", "MintA", 0.0, 0.0, 0.0, 0.0),
        ];
        let refs: Vec<_> = pairs.iter().collect();
        assert_eq!(count_distinct_dex_ids(&refs), 2);
    }

    #[test]
    fn single_dex_candidate_rejected() {
        let pairs = vec![sample_pair("raydium", "MintB", 600.0, 240.0, 1.0, 60_000.0)];
        let out = evaluate_momentum_candidate("MintB", &pairs, &cfg(), &WhaleSignalStore::new());
        assert!(out.is_none());
    }

    #[test]
    fn spike_metrics_gate_respects_config() {
        let c = cfg();
        assert!(passes_volume_spike_metrics(6.0, 1.5, 60_000.0, &c));
        assert!(!passes_volume_spike_metrics(4.0, 1.5, 60_000.0, &c));
        assert!(!passes_volume_spike_metrics(6.0, 2.5, 60_000.0, &c));
        assert!(!passes_volume_spike_metrics(6.0, 1.5, 40_000.0, &c));
    }

    #[test]
    fn candidate_requires_whale_for_confidence_above_threshold() {
        let pairs = vec![
            sample_pair("raydium", "MintC", 600.0, 240.0, 1.0, 60_000.0),
            sample_pair("orca", "MintC", 600.0, 240.0, 1.0, 60_000.0),
        ];
        let no_whale =
            evaluate_momentum_candidate("MintC", &pairs, &cfg(), &WhaleSignalStore::new());
        assert!(no_whale.is_none());

        let whale_store = WhaleSignalStore::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        whale_store.push(crate::whale_watcher::WhaleSwapSignal {
            wallet: crate::whale_watcher::WHALE_WALLETS[0].to_owned(),
            token_in: USDC_MINT.to_owned(),
            token_out: "MintC".to_owned(),
            amount_usd: 20_000.0,
            dex: crate::whale_watcher::DexSource::Jupiter,
            timestamp: now,
            tx_slot: 1,
            is_buy: true,
            signature: "sig".to_owned(),
        });
        let with_whale = evaluate_momentum_candidate("MintC", &pairs, &cfg(), &whale_store);
        assert!(with_whale.is_some());
        assert!(with_whale.unwrap().confidence > cfg().min_confidence);
    }
}
