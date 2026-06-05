//! Intelligence API — whale detection + wallet tracking + WS broker.
//! Consumes `data-layer` pipeline on :8090/intelligence.

mod broker;
mod contracts;
mod pipeline;
mod router;
mod routes;
mod whale;
mod wallet;

use crossbeam_channel::unbounded;
use data_layer::spawn_pipeline;
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::broker::spawn_batcher;
use crate::contracts::IntelligenceBatch;
use crate::pipeline::spawn_intelligence_pipeline;

#[derive(Clone)]
pub struct AppState {
    pub batch_tx: broadcast::Sender<IntelligenceBatch>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let handles = spawn_pipeline()?;
    let threshold = handles.config.whale_threshold_sol;

    let (msg_tx, msg_rx) = unbounded();
    let (batch_tx, _) = broadcast::channel::<IntelligenceBatch>(512);

    spawn_intelligence_pipeline(handles.ws_rx, msg_tx, threshold);
    spawn_batcher(msg_rx, batch_tx.clone());

    let port: u16 = std::env::var("INTELLIGENCE_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8090);

    let state = AppState { batch_tx };
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("intelligence-api listening on http://{addr}");
    info!("WebSocket stream at ws://{addr}/intelligence");

    let app = router::build(state);
    axum::serve(listener, app).await?;

    Ok(())
}
