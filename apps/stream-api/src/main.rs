//! Low-latency Solana stream API — mock ingestion, in-memory market state, batched WebSocket.
//!
//! Default: `http://0.0.0.0:8080/stream`

mod bridge;
mod broker;
mod contracts;
mod external_pollers;
mod hub_client;
mod ingestion;
mod market;
mod pricing;
mod pump_pollers;
mod router;
mod routes;
mod scanner_pollers;

use std::sync::Arc;

use crossbeam_channel::unbounded;
use data_layer::spawn_pipeline;
use signal_bus::SignalBus;
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Optional OpenTelemetry tracing initialization.
#[allow(dead_code)]
fn try_init_otlp() -> bool {
    // Only attempt to initialize when collector endpoint is set.
    let configured = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok()
        || std::env::var("OTEL_COLLECTOR_URL").is_ok();
    if !configured {
        return false;
    }

    // If crate built with `otel` feature, enable the full OTLP pipeline.
    #[cfg(feature = "otel")]
    {
        use opentelemetry::sdk::trace as sdktrace;

        let tracer = match opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(opentelemetry_otlp::new_exporter().http())
            .with_trace_config(sdktrace::config().with_sampler(sdktrace::Sampler::AlwaysOn))
            .install_batch(opentelemetry::runtime::Tokio)
        {
            Ok(t) => t,
            Err(e) => {
                eprintln!("failed to init opentelemetry: {e}");
                return false;
            }
        };

        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
            .with(telemetry)
            .init();

        return true;
    }

    #[cfg(not(feature = "otel"))]
    {
        eprintln!("OTEL configured but crate built without 'otel' feature; enable feature \"otel\" to export traces");
        return false;
    }
}

use crate::bridge::spawn_data_layer_bridge;
use crate::contracts::WSBatchFrame;
use crate::hub_client::{spawn_hub_mirror, spawn_signal_bus_fanout};
use crate::ingestion::spawn_mock_ingestion;
use crate::market::spawn_market_engine;

#[derive(Clone)]
pub struct AppState {
    pub batch_tx: broadcast::Sender<WSBatchFrame>,
    pub market: Arc<market::MarketEngineHandle>,
    pub signal_bus: Arc<SignalBus>,
    pub scanners: signals::ScannerStore,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize OTLP tracing if configured, otherwise set a default subscriber.
    if !try_init_otlp() {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
            .init();
    }

    let (swap_tx, swap_rx) = unbounded();
    let (ws_tx, ws_rx) = unbounded();
    let (batch_tx, _) = broadcast::channel::<WSBatchFrame>(512);
    let signal_bus = Arc::new(SignalBus::with_defaults());
    signal_bus.load_persisted().await;

    let market = spawn_market_engine(swap_rx, ws_tx.clone());
    broker::spawn_batcher(ws_rx, batch_tx.clone());
    spawn_signal_bus_fanout(signal_bus.clone(), ws_tx.clone());
    external_pollers::spawn_optional_external_pollers();
    pricing::spawn_jupiter_price_poller(ws_tx.clone());

    let pump_swap_tx = swap_tx.clone();

    let hub_url = std::env::var("SIGNAL_HUB_URL").ok().filter(|s| !s.is_empty());
    let use_mock = std::env::var("STREAM_USE_MOCK")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if let Some(hub) = hub_url {
        spawn_hub_mirror(hub, signal_bus.clone(), swap_tx, ws_tx.clone());
        info!("stream-api mirroring control-api signal hub (SIGNAL_HUB_URL)");
    } else if use_mock {
        let mock_interval: u64 = std::env::var("MOCK_SWAP_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(80);
        let _ingestion = spawn_mock_ingestion(swap_tx, mock_interval);
        info!("stream-api using legacy mock ingestion (STREAM_USE_MOCK=1)");
    } else {
        let handles = spawn_pipeline()?;
        spawn_data_layer_bridge(handles.ws_rx, signal_bus.clone(), swap_tx);
        info!("stream-api bridged to data-layer pipeline + signal-bus");
    }

    let port: u16 = std::env::var("STREAM_API_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);

    let scanners = signals::ScannerStore::new();
    scanner_pollers::spawn_scanner_pollers(scanners.clone());
    pump_pollers::spawn_pump_pollers(pump_swap_tx, ws_tx.clone(), scanners.clone());

    let state = AppState {
        batch_tx,
        market: Arc::new(market),
        signal_bus,
        scanners,
    };

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("stream-api listening on http://{addr}");
    info!("WebSocket stream at ws://{addr}/stream (schema v{})", contracts::SCHEMA_VERSION);

    let app = router::build(state);
    axum::serve(listener, app).await?;

    Ok(())
}
