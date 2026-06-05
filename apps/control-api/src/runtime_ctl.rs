//! Manages the autonomous trading runtime lifecycle.

use std::sync::Arc;

use autonomous::{spawn_autonomous_runtime, AutonomousCallbacks, RuntimeSnapshot};
use config::SystemConfig;
use signals::SignalEvent;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::{dto::SignalEventDto, state::AppState};

pub struct RuntimeController {
    pub cancel: CancellationToken,
    pub handle: JoinHandle<()>,
    pub snapshot: Arc<RwLock<RuntimeSnapshot>>,
}

impl RuntimeController {
    pub async fn stop(self) {
        self.cancel.cancel();
        let _ = self.handle.await;
    }
}

pub async fn start_runtime(state: &AppState) -> Result<(), String> {
    let mut cfg: SystemConfig = state.config.read().await.clone();
    let tuned = autonomous::tuned_config();
    cfg.scalper = tuned.scalper;
    cfg.signal_engine = tuned.signal_engine;
    cfg.execution = tuned.execution;

    let snapshot = Arc::new(RwLock::new(RuntimeSnapshot::default()));
    let cancel = CancellationToken::new();

    let app = state.clone();
    let callbacks = AutonomousCallbacks {
        on_signal: Some(Arc::new(move |signal: SignalEvent| {
            let app = app.clone();
            let dto = SignalEventDto::from(&signal);
            tokio::spawn(async move {
                app.ingest_signal(dto).await;
            });
        })),
    };

    let handle = spawn_autonomous_runtime(
        Arc::new(cfg),
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
            handle,
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
        RuntimeSnapshot::default()
    }
}
