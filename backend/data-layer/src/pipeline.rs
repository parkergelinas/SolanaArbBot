//! End-to-end pipeline orchestration.

use crossbeam_channel::{unbounded, Receiver, Sender};
use tracing::info;

use crate::enrich::EnrichmentStore;
use crate::ingest;
use crate::normalize;
use crate::parser;
use crate::router::EventRouter;
use crate::types::{PipelineEvent, RawUpdate};

pub struct PipelineConfig {
    pub whale_threshold_sol: f64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            whale_threshold_sol: std::env::var("WHALE_THRESHOLD_SOL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10.0),
        }
    }
}

pub struct PipelineHandles {
    pub whale_rx: Receiver<PipelineEvent>,
    pub wallet_rx: Receiver<PipelineEvent>,
    pub ws_rx: Receiver<PipelineEvent>,
    pub config: PipelineConfig,
}

/// Spawn full ingest → parse → normalize → enrich → route pipeline.
pub fn spawn_pipeline() -> anyhow::Result<PipelineHandles> {
    let config = PipelineConfig::default();
    let (raw_tx, raw_rx) = unbounded::<RawUpdate>();
    let (enriched_tx, enriched_rx) = unbounded::<PipelineEvent>();
    let (whale_tx, whale_rx) = unbounded();
    let (wallet_tx, wallet_rx) = unbounded();
    let (ws_tx, ws_rx) = unbounded();

    ingest::spawn_default(raw_tx)?;

    let enricher = EnrichmentStore::new();
    std::thread::spawn(move || {
        while let Ok(raw) = raw_rx.recv() {
            for leg in parser::parse_swap_legs(&raw) {
                let swap = normalize::normalize(&raw, &leg);
                let enriched = enricher.enrich(swap, raw.slot);
                let _ = enriched_tx.send(PipelineEvent::EnrichedSwap(enriched));
            }
        }
    });

    EventRouter::fan_out(enriched_rx, whale_tx, wallet_tx, ws_tx);

    info!(
        whale_threshold_sol = config.whale_threshold_sol,
        "data-layer pipeline online"
    );

    Ok(PipelineHandles {
        whale_rx,
        wallet_rx,
        ws_rx,
        config,
    })
}
