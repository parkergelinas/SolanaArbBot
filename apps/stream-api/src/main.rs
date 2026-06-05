//! Low-latency Solana stream API — mock ingestion, in-memory market state, batched WebSocket.
//!
//! Default: `http://0.0.0.0:8080/stream`

mod broker;
mod contracts;
mod ingestion;
mod market;
mod router;
mod routes;

use std::sync::Arc;

use crossbeam_channel::unbounded;
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::contracts::WSBatchFrame;
use crate::ingestion::spawn_mock_ingestion;
use crate::market::spawn_market_engine;

#[derive(Clone)]
pub struct AppState {
    pub batch_tx: broadcast::Sender<WSBatchFrame>,
    pub market: Arc<market::MarketEngineHandle>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let (swap_tx, swap_rx) = unbounded();
    let (ws_tx, ws_rx) = unbounded();
    let (batch_tx, _) = broadcast::channel::<WSBatchFrame>(512);

    let market = spawn_market_engine(swap_rx, ws_tx);
    broker::spawn_batcher(ws_rx, batch_tx.clone());

    let mock_interval: u64 = std::env::var("MOCK_SWAP_INTERVAL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(80);

    let _ingestion = spawn_mock_ingestion(swap_tx, mock_interval);

    let port: u16 = std::env::var("STREAM_API_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);

    let state = AppState {
        batch_tx,
        market: Arc::new(market),
    };

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("stream-api listening on http://{addr}");
    info!("WebSocket stream at ws://{addr}/stream (schema v{})", contracts::SCHEMA_VERSION);

    let app = router::build(state);
    axum::serve(listener, app).await?;

    Ok(())
}
