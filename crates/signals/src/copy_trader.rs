//! Copy-trading execution — mirrors qualified whale buys and optional exits.
//!
//! Consumes [`CopySignal`]s from [`crate::whale_watcher`] and submits Jupiter
//! swaps (or logs in dry-run). Reuses the execution hotpath Jito/RPC routing
//! pattern when live trading is enabled.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use config::{CopyTradingConfig, DataSourcesConfig, FeatureFlags};
use crossbeam_channel::{Receiver, Sender};
use pricing::JUPITER_SWAP_V1_BASE;
use tracing::{info, warn};

use crate::wallet_scoring::QualifiedWalletSet;
use crate::whale_watcher::{short_wallet, DexSource, USDC_MINT};

/// Jupiter swap slippage for copy trades (1.5%).
pub const COPY_SLIPPAGE_BPS: u32 = 150;

/// Jupiter Swap API v1 base (cold-path submission).
const JUPITER_SWAP_API: &str = JUPITER_SWAP_V1_BASE;

/// Whale swap event eligible for copy-trading.
#[derive(Clone, Debug, PartialEq)]
pub struct CopySignal {
    pub wallet: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_usd: f64,
    pub dex: DexSource,
    pub timestamp: i64,
    /// Solana slot when the whale transaction landed.
    pub tx_slot: u64,
    /// Slot when we detected the signal (for staleness).
    pub detected_slot: u64,
    pub is_buy: bool,
    pub signature: String,
}

/// Why a copy trade was skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopySkipReason {
    Stale,
    ExitTrap,
    MaxConcurrent,
    NotQualified,
    BelowMinUsd,
    DryRun,
}

/// Planned copy trade before submission.
#[derive(Clone, Debug, PartialEq)]
pub struct CopyExecutionPlan {
    pub token_in: String,
    pub token_out: String,
    pub copy_amount_usd: f64,
    pub slippage_bps: u32,
    pub is_exit: bool,
}

/// Tracks recent whale sells per wallet for exit-trap guard.
#[derive(Clone, Default)]
pub struct RecentSalesTracker {
    inner: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl RecentSalesTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_sale(&self, wallet: &str, token_mint: &str) {
        let mut g = self.inner.write().expect("lock");
        g.entry(wallet.to_owned())
            .or_default()
            .insert(token_mint.to_owned());
    }

    pub fn contains(&self, wallet: &str, token_mint: &str) -> bool {
        self.inner
            .read()
            .expect("lock")
            .get(wallet)
            .is_some_and(|set| set.contains(token_mint))
    }

    pub fn clear_sale(&self, wallet: &str, token_mint: &str) {
        if let Some(set) = self.inner.write().expect("lock").get_mut(wallet) {
            set.remove(token_mint);
        }
    }
}

/// Open copy position keyed by token mint.
#[derive(Clone, Debug)]
struct CopyPosition {
    token_mint: String,
    wallet: String,
    size_usd: f64,
    opened_slot: u64,
}

/// Runtime state for the copy executor.
#[derive(Clone)]
pub struct CopyTraderState {
    pub recent_sales: RecentSalesTracker,
    positions: Arc<RwLock<HashMap<String, CopyPosition>>>,
    active_copies: Arc<RwLock<u32>>,
    current_slot: Arc<RwLock<u64>>,
}

impl CopyTraderState {
    pub fn new(recent_sales: RecentSalesTracker) -> Self {
        Self {
            recent_sales,
            positions: Arc::new(RwLock::new(HashMap::new())),
            active_copies: Arc::new(RwLock::new(0)),
            current_slot: Arc::new(RwLock::new(0)),
        }
    }

    pub fn set_current_slot(&self, slot: u64) {
        *self.current_slot.write().expect("lock") = slot;
    }

    pub fn current_slot(&self) -> u64 {
        *self.current_slot.read().expect("lock")
    }

    fn active_count(&self) -> u32 {
        *self.active_copies.read().expect("lock")
    }

    fn try_reserve_slot(&self, max: u32) -> bool {
        let mut g = self.active_copies.write().expect("lock");
        if *g >= max {
            return false;
        }
        *g += 1;
        true
    }

    fn release_slot(&self) {
        let mut g = self.active_copies.write().expect("lock");
        *g = g.saturating_sub(1);
    }

    fn open_position(&self, token: &str, wallet: &str, size_usd: f64, slot: u64) {
        self.positions.write().expect("lock").insert(
            token.to_owned(),
            CopyPosition {
                token_mint: token.to_owned(),
                wallet: wallet.to_owned(),
                size_usd,
                opened_slot: slot,
            },
        );
    }

    fn close_position(&self, token: &str) -> Option<CopyPosition> {
        self.positions.write().expect("lock").remove(token)
    }

    pub fn has_position(&self, token: &str) -> bool {
        self.positions.read().expect("lock").contains_key(token)
    }

    #[cfg(test)]
    pub fn open_position_for_test(&self, token: &str, wallet: &str, size_usd: f64, slot: u64) {
        self.open_position(token, wallet, size_usd, slot);
    }
}

/// Compute copy amount and validate guards (unit-testable).
pub fn plan_copy(
    signal: &CopySignal,
    cfg: &CopyTradingConfig,
    state: &CopyTraderState,
    current_slot: u64,
) -> Result<CopyExecutionPlan, CopySkipReason> {
    if !signal.is_buy {
        return Err(CopySkipReason::BelowMinUsd);
    }
    if signal.amount_usd < cfg.min_whale_trade_usd {
        return Err(CopySkipReason::BelowMinUsd);
    }

    let slots_elapsed = current_slot.saturating_sub(signal.tx_slot);
    if slots_elapsed > cfg.max_staleness_slots {
        return Err(CopySkipReason::Stale);
    }

    if state
        .recent_sales
        .contains(&signal.wallet, &signal.token_out)
    {
        return Err(CopySkipReason::ExitTrap);
    }

    if state.active_count() >= cfg.max_concurrent_copies {
        return Err(CopySkipReason::MaxConcurrent);
    }

    let copy_amount = (signal.amount_usd * cfg.copy_ratio).min(cfg.max_copy_usd);

    Ok(CopyExecutionPlan {
        token_in: signal.token_in.clone(),
        token_out: signal.token_out.clone(),
        copy_amount_usd: copy_amount,
        slippage_bps: COPY_SLIPPAGE_BPS,
        is_exit: false,
    })
}

/// Execute (or log) a copy trade from a whale buy signal.
pub fn execute_copy(
    signal: &CopySignal,
    cfg: &CopyTradingConfig,
    state: &CopyTraderState,
    features: &FeatureFlags,
    data_sources: &DataSourcesConfig,
    current_slot: u64,
) -> Result<CopyExecutionPlan, CopySkipReason> {
    let plan = plan_copy(signal, cfg, state, current_slot)?;

    if features.dry_run || !features.enable_live_trading {
        info!(
            wallet = %short_wallet(&signal.wallet),
            token_out = %short_wallet(&signal.token_out),
            copy_usd = plan.copy_amount_usd,
            slippage_bps = plan.slippage_bps,
            dex = signal.dex.as_str(),
            signature = %signal.signature,
            dry_run = features.dry_run,
            "CopySignal (dry-run — no swap submitted)"
        );
        return Err(CopySkipReason::DryRun);
    }

    if !state.try_reserve_slot(cfg.max_concurrent_copies) {
        return Err(CopySkipReason::MaxConcurrent);
    }

    info!(
        wallet = %short_wallet(&signal.wallet),
        token_in = %short_wallet(&plan.token_in),
        token_out = %short_wallet(&plan.token_out),
        copy_usd = plan.copy_amount_usd,
        slippage_bps = plan.slippage_bps,
        jupiter = JUPITER_SWAP_API,
        prefer_jito = features.enable_jito,
        "CopySignal executing Jupiter swap"
    );

    state.open_position(
        &plan.token_out,
        &signal.wallet,
        plan.copy_amount_usd,
        current_slot,
    );

    // Cold-path Jupiter swap + optional Jito bundle (placeholder I/O).
    submit_copy_swap(&plan, data_sources, features.enable_jito);
    state.release_slot();

    Ok(plan)
}

/// Mirror whale exit — sell our position when whale sells the same token.
pub fn execute_mirror_exit(
    signal: &CopySignal,
    cfg: &CopyTradingConfig,
    state: &CopyTraderState,
    features: &FeatureFlags,
    data_sources: &DataSourcesConfig,
) -> bool {
    if !cfg.mirror_exits || signal.is_buy {
        return false;
    }

    let sold_token = &signal.token_in;
    if !state.has_position(sold_token) {
        return false;
    }

    let pos = state.close_position(sold_token).expect("position exists");

    if features.dry_run || !features.enable_live_trading {
        info!(
            wallet = %short_wallet(&signal.wallet),
            token = %short_wallet(sold_token),
            size_usd = pos.size_usd,
            "Mirror exit (dry-run — no sell submitted)"
        );
        return true;
    }

    let plan = CopyExecutionPlan {
        token_in: sold_token.clone(),
        token_out: USDC_MINT.to_owned(),
        copy_amount_usd: pos.size_usd,
        slippage_bps: COPY_SLIPPAGE_BPS,
        is_exit: true,
    };

    info!(
        wallet = %short_wallet(&signal.wallet),
        token = %short_wallet(sold_token),
        size_usd = pos.size_usd,
        "Mirror exit executing Jupiter sell"
    );
    submit_copy_swap(&plan, data_sources, features.enable_jito);
    true
}

fn submit_copy_swap(plan: &CopyExecutionPlan, _data_sources: &DataSourcesConfig, use_jito: bool) {
    // Placeholder: wires to Jupiter quote/swap API + execution hotpath Jito/RPC.
    let _ = (
        JUPITER_SWAP_API,
        plan.slippage_bps,
        use_jito,
        &plan.token_in,
        &plan.token_out,
        plan.copy_amount_usd,
    );
}

/// Bounded channel for copy signals.
pub fn copy_signal_channel(capacity: usize) -> (Sender<CopySignal>, Receiver<CopySignal>) {
    crossbeam_channel::bounded(capacity)
}

/// Spawn the copy-trading executor loop.
pub fn spawn_copy_trader(
    rx: Receiver<CopySignal>,
    cfg: Arc<CopyTradingConfig>,
    features: Arc<FeatureFlags>,
    data_sources: Arc<DataSourcesConfig>,
    state: CopyTraderState,
    qualified: Option<QualifiedWalletSet>,
) {
    info!(
        copy_ratio = cfg.copy_ratio,
        max_copy_usd = cfg.max_copy_usd,
        max_concurrent = cfg.max_concurrent_copies,
        mirror_exits = cfg.mirror_exits,
        "copy trader executor started"
    );

    tokio::spawn(async move {
        while let Ok(signal) = rx.recv() {
            if let Some(ref q) = qualified {
                if !q.is_tracked(&signal.wallet) {
                    warn!(
                        wallet = %short_wallet(&signal.wallet),
                        "CopySignal skipped — wallet not qualified"
                    );
                    continue;
                }
            }

            state.set_current_slot(signal.detected_slot);

            if signal.is_buy {
                if let Err(reason) = execute_copy(
                    &signal,
                    &cfg,
                    &state,
                    &features,
                    &data_sources,
                    signal.detected_slot,
                ) {
                    if reason != CopySkipReason::DryRun {
                        warn!(
                            wallet = %short_wallet(&signal.wallet),
                            ?reason,
                            "CopySignal skipped"
                        );
                    }
                }
            } else {
                state
                    .recent_sales
                    .record_sale(&signal.wallet, &signal.token_in);
                execute_mirror_exit(&signal, &cfg, &state, &features, &data_sources);
            }
        }
    });
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Emit a copy signal to the signal-bus format (optional dashboard bridge).
pub fn copy_signal_to_bus_payload(signal: &CopySignal) -> serde_json::Value {
    serde_json::json!({
        "kind": "whale_copy",
        "wallet": short_wallet(&signal.wallet),
        "token_in": signal.token_in,
        "token_out": signal.token_out,
        "amount_usd": signal.amount_usd,
        "dex": signal.dex.as_str(),
        "is_buy": signal.is_buy,
        "signature": signal.signature,
        "tx_slot": signal.tx_slot,
        "detected_slot": signal.detected_slot,
        "timestamp_ms": unix_now_ms(),
        "strategy_tag": "whale_copy_candidate",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::whale_watcher::SOL_MINT;

    fn buy_signal(slot: u64) -> CopySignal {
        CopySignal {
            wallet: "AVAZvHLR2PcWpDf8BXY4rVxNHYRBytycHkcB5z5QNXYm".to_owned(),
            token_in: USDC_MINT.to_owned(),
            token_out: "TokenMint111".to_owned(),
            amount_usd: 10_000.0,
            dex: DexSource::Jupiter,
            timestamp: 1_700_000_000,
            tx_slot: slot,
            detected_slot: slot,
            is_buy: true,
            signature: "sig123".to_owned(),
        }
    }

    fn cfg() -> CopyTradingConfig {
        CopyTradingConfig::default()
    }

    fn state() -> CopyTraderState {
        CopyTraderState::new(RecentSalesTracker::new())
    }

    #[test]
    fn plan_copy_computes_ratio_capped() {
        let signal = buy_signal(100);
        let plan = plan_copy(&signal, &cfg(), &state(), 100).expect("plan");
        assert!((plan.copy_amount_usd - 50.0).abs() < f64::EPSILON);
        assert_eq!(plan.slippage_bps, COPY_SLIPPAGE_BPS);
    }

    #[test]
    fn staleness_guard_skips_old_signals() {
        let signal = buy_signal(90);
        let err = plan_copy(&signal, &cfg(), &state(), 100).expect_err("stale");
        assert_eq!(err, CopySkipReason::Stale);
    }

    #[test]
    fn exit_trap_guard_skips_recent_sales() {
        let st = state();
        st.recent_sales
            .record_sale("AVAZvHLR2PcWpDf8BXY4rVxNHYRBytycHkcB5z5QNXYm", "TokenMint111");
        let signal = buy_signal(100);
        let err = plan_copy(&signal, &cfg(), &st, 100).expect_err("exit trap");
        assert_eq!(err, CopySkipReason::ExitTrap);
    }

    #[test]
    fn max_concurrent_guard_blocks() {
        let st = state();
        *st.active_copies.write().expect("lock") = 3;
        let signal = buy_signal(100);
        let err = plan_copy(&signal, &cfg(), &st, 100).expect_err("max concurrent");
        assert_eq!(err, CopySkipReason::MaxConcurrent);
    }

    #[test]
    fn execute_copy_dry_run_logs_without_trade() {
        let features = FeatureFlags::default();
        let data = DataSourcesConfig::default();
        let signal = buy_signal(100);
        let err = execute_copy(&signal, &cfg(), &state(), &features, &data, 100)
            .expect_err("dry run");
        assert_eq!(err, CopySkipReason::DryRun);
    }

    #[test]
    fn mirror_exit_dry_run_when_whale_sells() {
        let st = state();
        st.open_position_for_test("TokenMint111", "whale1", 50.0, 100);

        let sell = CopySignal {
            wallet: "whale1".to_owned(),
            token_in: "TokenMint111".to_owned(),
            token_out: SOL_MINT.to_owned(),
            amount_usd: 8_000.0,
            dex: DexSource::Raydium,
            timestamp: 1_700_000_000,
            tx_slot: 101,
            detected_slot: 101,
            is_buy: false,
            signature: "sig456".to_owned(),
        };

        let features = FeatureFlags::default();
        let data = DataSourcesConfig::default();
        assert!(execute_mirror_exit(&sell, &cfg(), &st, &features, &data));
        assert!(!st.has_position("TokenMint111"));
    }
}
