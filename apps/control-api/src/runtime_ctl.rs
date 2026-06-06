//! Manages the autonomous trading runtime lifecycle.

use std::sync::Arc;

use autonomous::{
    spawn_autonomous_runtime, AutonomousCallbacks, ExternalIngestionBuffer, LiveExecutionContext,
    RuntimeSnapshot, TradeEmit,
};
use config::{live_trading_confirm_env_set, SystemConfig};
use pricing::TokenQualityFilter;
use signal_bus::SignalBus;
use signals::{spawn_quote_arb_publisher, QuoteArbSignal, SignalEvent};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use wallet::{guard, make_rpc, BalanceMonitor, WalletKeypair};

use crate::{
    dto::{SignalEventDto, TradeEventDto},
    signal_bridge::{publish_quote_arb_signal, spawn_signal_bus_bridge},
    state::AppState,
};

pub struct RuntimeController {
    pub cancel: CancellationToken,
    pub bridge_cancel: CancellationToken,
    pub quote_arb_cancel: CancellationToken,
    pub handle: JoinHandle<()>,
    pub bridge_handle: JoinHandle<()>,
    pub quote_arb_handle: Option<JoinHandle<()>>,
    pub snapshot: Arc<RwLock<RuntimeSnapshot>>,
}

impl RuntimeController {
    pub async fn stop(self) {
        self.quote_arb_cancel.cancel();
        self.bridge_cancel.cancel();
        self.cancel.cancel();
        if let Some(handle) = self.quote_arb_handle {
            let _ = handle.await;
        }
        let _ = self.bridge_handle.await;
        let _ = self.handle.await;
    }
}

fn live_mode_requested(user_cfg: &SystemConfig) -> bool {
    user_cfg.features.enable_live_trading
        && !user_cfg.features.dry_run
        && live_trading_confirm_env_set()
}

/// Merge user config with tuned scalper defaults; preserve user feature flags when valid.
pub fn merge_runtime_config(user_cfg: &SystemConfig) -> Result<SystemConfig, String> {
    let mut cfg = autonomous::tuned_config();

    cfg.strategy = user_cfg.strategy.clone();
    cfg.features = user_cfg.features.clone();
    cfg.wallet = user_cfg.wallet.clone();
    cfg.rpc = user_cfg.rpc.clone();
    cfg.data_sources = user_cfg.data_sources.clone();
    cfg.quote_arb = user_cfg.quote_arb.clone();
    cfg.arbitrage = user_cfg.arbitrage.clone();
    cfg.portfolio = user_cfg.portfolio.clone();
    cfg.risk = user_cfg.risk.clone();
    cfg.execution = user_cfg.execution.clone();
    cfg.signal_engine = user_cfg.signal_engine.clone();

    cfg.scalper.take_profit_pct = user_cfg.scalper.take_profit_pct;
    cfg.scalper.stop_loss_pct = user_cfg.scalper.stop_loss_pct;
    cfg.execution.min_profit_threshold_usd = user_cfg.execution.min_profit_threshold_usd;
    cfg.execution.max_loss_per_trade_usd = user_cfg.execution.max_loss_per_trade_usd;
    cfg.signal_engine.whale_threshold_usd = user_cfg.signal_engine.whale_threshold_usd;
    cfg.signal_engine.signal_min_confidence = user_cfg.signal_engine.signal_min_confidence;
    cfg.signal_engine.signal_min_strength = user_cfg.signal_engine.signal_min_strength;

    if live_mode_requested(user_cfg) {
        cfg.features.validate()?;
        cfg.validate()?;
    } else {
        cfg.features.dry_run = true;
        cfg.features.enable_live_trading = false;
    }

    Ok(cfg)
}

async fn bootstrap_live_context(cfg: &SystemConfig) -> Result<Arc<LiveExecutionContext>, String> {
    guard::require_live_mode(cfg).map_err(|e| e.to_string())?;

    let wallet = WalletKeypair::load_wallet_key(cfg).map_err(|e| e.to_string())?;
    let rpc = make_rpc(&cfg.wallet.rpc_endpoint, &cfg.wallet.commitment)
        .map_err(|e| e.to_string())?;

    if cfg.wallet.validate_network_on_start {
        guard::validate_network(rpc.as_ref(), &cfg.wallet)
            .await
            .map_err(|e| e.to_string())?;
    }

    let monitor = BalanceMonitor::new(wallet.pubkey(), &cfg.wallet, Arc::clone(&rpc)).await;
    monitor.refresh_sol().await.map_err(|e| e.to_string())?;
    let report = monitor.balance_report().await;
    guard::require_sufficient_balance(&report).map_err(|e| e.to_string())?;

    tracing::info!(
        wallet = %report.pubkey,
        sol = report.sol_balance,
        network = %cfg.wallet.expected_network,
        "live wallet bootstrapped"
    );

    Ok(Arc::new(LiveExecutionContext {
        wallet: Arc::new(wallet),
        rpc,
        swap_base: cfg.data_sources.jupiter_swap.clone(),
        slippage_bps: cfg.execution.max_slippage_bps as u32,
    }))
}

fn spawn_quote_arb_bridge(
    cfg: &SystemConfig,
    signal_bus: Arc<SignalBus>,
    cancel: CancellationToken,
) -> Option<JoinHandle<()>> {
    if !cfg.strategy.quote_arb || !cfg.quote_arb.enabled {
        return None;
    }

    let quote_cfg = Arc::new(cfg.quote_arb.clone());
    let data_sources = Arc::new(cfg.data_sources.clone());
    let quality = TokenQualityFilter::new();
    let bus = signal_bus;

    Some(spawn_quote_arb_publisher(
        quote_cfg,
        data_sources,
        quality,
        cancel,
        move |sig: QuoteArbSignal| {
            let bus = bus.clone();
            tokio::spawn(async move {
                publish_quote_arb_signal(&bus, &sig).await;
            });
        },
    ))
}

pub async fn start_runtime(state: &AppState) -> Result<(), String> {
    let user_cfg = state.config.read().await.clone();
    let cfg = merge_runtime_config(&user_cfg)?;
    let live_ctx = if live_mode_requested(&user_cfg) {
        Some(bootstrap_live_context(&cfg).await?)
    } else {
        None
    };

    *state.config.write().await = cfg.clone();

    {
        let mut strategy = state.strategy.write().await;
        strategy.sync_from_config(&cfg);
        if let Some(t) = strategy.set_running(true) {
            tracing::info!(
                from = t.from_mode.as_str(),
                to = t.to_mode.as_str(),
                strategies = ?t.active_strategies,
                ingestion = t.ingestion_mode.as_str(),
                live = live_ctx.is_some(),
                "strategy runtime transition on start"
            );
        }
    }

    let snapshot = Arc::new(RwLock::new(RuntimeSnapshot::default()));
    let cancel = CancellationToken::new();
    let bridge_cancel = CancellationToken::new();
    let quote_arb_cancel = CancellationToken::new();

    let bridge_handle = spawn_signal_bus_bridge(
        Arc::clone(&state.signal_bus),
        Arc::clone(&state.external_ingest),
        bridge_cancel.clone(),
    );

    let quote_arb_handle = spawn_quote_arb_bridge(&cfg, state.signal_bus.clone(), quote_arb_cancel.clone());

    let app_signal = state.clone();
    let app_trade = state.clone();
    let callbacks = AutonomousCallbacks {
        on_signal: Some(Arc::new(move |signal: SignalEvent| {
            let app = app_signal.clone();
            let dto = SignalEventDto::from(&signal);
            tokio::spawn(async move {
                app.ingest_signal(dto).await;
            });
        })),
        on_trade: Some(Arc::new(move |trade: TradeEmit| {
            let app = app_trade.clone();
            let dto = TradeEventDto::from(&trade);
            tokio::spawn(async move {
                app.ingest_trade(dto).await;
            });
        })),
    };

    let handle = spawn_autonomous_runtime(
        Arc::clone(&state.config),
        Arc::clone(&state.strategy),
        Arc::clone(&state.external_ingest),
        Arc::clone(&snapshot),
        live_ctx,
        cancel.clone(),
        callbacks,
    );

    {
        let mut rt = state.runtime.lock().await;
        if rt.is_some() {
            return Err("runtime already active".to_owned());
        }
        *rt = Some(RuntimeController {
            cancel,
            bridge_cancel,
            quote_arb_cancel,
            handle,
            bridge_handle,
            quote_arb_handle,
            snapshot,
        });
    }

    {
        let mut sys = state.sys.lock().await;
        sys.running = true;
    }

    Ok(())
}

pub async fn stop_runtime(state: &AppState) -> Result<(), String> {
    let controller = {
        let mut rt = state.runtime.lock().await;
        rt.take()
    };

    let Some(controller) = controller else {
        return Err("runtime not active".to_owned());
    };

    controller.stop().await;

    {
        let mut strategy = state.strategy.write().await;
        if let Some(t) = strategy.set_running(false) {
            tracing::info!(
                from = t.from_mode.as_str(),
                to = t.to_mode.as_str(),
                "strategy runtime transition on stop"
            );
        }
    }

    {
        let mut external = state.external_ingest.write().await;
        *external = ExternalIngestionBuffer::default();
    }

    {
        let mut sys = state.sys.lock().await;
        sys.running = false;
    }

    Ok(())
}

pub async fn current_snapshot(state: &AppState) -> RuntimeSnapshot {
    let rt = state.runtime.lock().await;
    if let Some(ref controller) = *rt {
        controller.snapshot.read().await.clone()
    } else {
        let cfg = state.config.read().await;
        let strategy = state.strategy.read().await;
        let plan = strategy.plan_cycle(&cfg);
        let mut snap = RuntimeSnapshot::default();
        snap.runtime_mode = plan.runtime_mode.as_str().to_owned();
        snap.ingestion_mode = plan.ingestion.mode.as_str().to_owned();
        snap.active_strategies = plan.active_strategies;
        snap.mode = plan.runtime_mode.as_str().to_owned();
        snap
    }
}

pub async fn strategy_status(state: &AppState) -> (String, String, Vec<String>) {
    let cfg = state.config.read().await;
    let strategy = state.strategy.read().await;
    let plan = strategy.plan_cycle(&cfg);
    (
        plan.runtime_mode.as_str().to_owned(),
        plan.ingestion.mode.as_str().to_owned(),
        plan.active_strategies,
    )
}
