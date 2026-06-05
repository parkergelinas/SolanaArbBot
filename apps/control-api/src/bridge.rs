//! Data-layer → signal-bus bridge for control-api live swap feed.

use std::sync::Arc;

use crossbeam_channel::Receiver;
use data_layer::types::PipelineEvent;
use signal_bus::{from_enriched, SignalBus};
use tracing::info;

use crate::{dto::SignalEventDto, state::AppState};

pub fn spawn_data_layer_bridge(
    pipeline_rx: Receiver<PipelineEvent>,
    bus: Arc<SignalBus>,
    state: AppState,
) {
    std::thread::spawn(move || {
        let replayed = bus.replay_blocking();
        if !replayed.is_empty() {
            info!(
                count = replayed.len(),
                "control-api signal-bus replay available on bridge start"
            );
        }

        while let Ok(ev) = pipeline_rx.recv() {
            let enriched = ev.enriched();
            let live = from_enriched(enriched);
            if let Some(accepted) = bus.publish_blocking(live) {
                let dto = SignalEventDto::from(&accepted);
                let state = state.clone();
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        state.ingest_signal(dto).await;
                    });
                }
            }
        }
    });
}
