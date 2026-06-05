//! Manages the autonomous trading runtime lifecycle.

use std::sync::Arc;

use autonomous::{
    spawn_autonomous_runtime, AutonomousCallbacks, ExternalIngestionBuffer, RuntimeSnapshot,
    TradeEmit,
};
use config::SystemConfig;
use signals::SignalEvent;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::{
    dto::{SignalEventDto, TradeEventDto},
    signal_bridge::spawn_signal_bus_bridge,
    state::AppState,
};

pub struct RuntimeController {
    pub cancel: CancellationToken,
    pub bridge_cancel: CancellationToken,
    pub handle: JoinHandle<()>,
    pub bridge_handle: JoinHandle<()>,
    pub snapshot: Arc<RwLock<RuntimeSnapshot>>,
}

impl RuntimeController {
    pub async fn stop(self) {
        self.bridge_cancel.cancel();
        self.cancel.cancel();
        let _ = self.bridge_handle.await;
        let _ = self.handle.await;
    }
}

pub async fn start_runtime(state: &AppState) -> Result<(), String> {
    let mut cfg: SystemConfig = state.config.read().await.clone();
    let tuned = autonomous::tuned_config();
    cfg.scalper = tuned.scalper;
    cfg.signal_engine = tuned.signal_engine;
    cfg.execution = tuned.execution;

    {
        let mut strategy = state.strategy.write().await;
        strategy.sync_from_config(&cfg);
        if let Some(t) = strategy.set_running(true) {
            tracing::info!(
                from = t.from_mode.as_str(),
                to = t.to_mode.as_str(),
                strategies = ?t.active_strategies,
                ingestion = t.ingestion_mode.as_str(),
                "strategy runtime transition on start"
            );
        }
    }

    let snapshot = Arc::new(RwLock::new(RuntimeSnapshot::default()));
    let cancel = CancellationToken::new();
    let bridge_cancel = CancellationToken::new();

    let bridge_handle = spawn_signal_bus_bridge(
        Arc::clone(&state.signal_bus),
        Arc::clone(&state.external_ingest),
        bridge_cancel.clone(),
    );

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
            handle,
            bridge_handle,
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
