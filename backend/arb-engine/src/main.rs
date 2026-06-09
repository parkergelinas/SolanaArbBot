//! Arb engine service — mock prices, spread detection, WS broadcast.

use std::sync::Arc;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use crossbeam_channel::{bounded, unbounded, Receiver};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use arb_engine::{
    config::ArbConfig,
    ingest,
    router::ArbRouter,
    signal::TradeSignalOut,
    types::ArbSignal,
    pool_state::PoolStateEngine,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Arc::new(ArbConfig::from_env());
    info!(
        min_spread_pct = config.min_spread_pct,
        batch_ms = config.batch_interval_ms,
        ws_port = config.ws_port,
        "arb-engine starting"
    );

    let pool_engine = Arc::new(PoolStateEngine::new());

    let (price_tx, price_rx) = unbounded();
    let (detect_tx, detect_rx) = bounded::<()>(64);
    let (signal_tx, signal_rx) = unbounded::<TradeSignalOut>();
    let (arb_tx, arb_rx) = unbounded::<ArbSignal>();

    ingest::spawn_price_consumer(pool_engine.clone(), price_rx);
    ingest::spawn_batch_detector(config.clone(), pool_engine.clone(), detect_tx);
    let use_hub = std::env::var("SIGNAL_HUB_URL")
        .or_else(|_| std::env::var("CONTROL_API_URL"))
        .is_ok()
        || std::env::var("ARB_USE_SIGNAL_HUB")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false);

    let geyser_endpoint = std::env::var("GEYSER_GRPC_ENDPOINT").ok();

    if use_hub {
        ingest::spawn_signal_hub_price_feed(config.clone(), price_tx.clone());
        info!("arb-engine using signal-hub prices");
    } else if geyser_endpoint.is_some() {
        // Geyser endpoint is configured — use live Jupiter poll as stand-in until
        // the yellowstone feature is compiled in and account bytes flow through.
        info!("GEYSER_GRPC_ENDPOINT detected — starting live Jupiter price poll");
        ingest::spawn_live_price_poll(config.clone(), price_tx);
    } else {
        ingest::spawn_intelligence_stub(price_tx.clone());
        ingest::spawn_mock_price_feed(config.clone(), price_tx);
    }

    let router = Arc::new(ArbRouter::new(
        config.clone(),
        pool_engine.clone(),
        signal_tx,
        Some(arb_tx),
    ));

    spawn_detection_worker(router, detect_rx);
    bridge_signals_to_tokio(signal_rx);
    let ws_state = spawn_ws_broadcast(arb_rx);

    if config.ws_enabled {
        let app = Router::new()
            .route("/arb", get(ws_handler))
            .route("/health", get(|| async { "ok" }))
            .layer(CorsLayer::permissive())
            .with_state(ws_state);

        let addr = format!("0.0.0.0:{}", config.ws_port);
        info!("arb WS listening on {addr}/arb");
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
    } else {
        info!("arb-engine running (WS disabled)");
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}

fn spawn_detection_worker(router: Arc<ArbRouter>, detect_rx: Receiver<()>) {
    std::thread::spawn(move || {
        while detect_rx.recv().is_ok() {
            router.run_detection_cycle();
        }
    });
}

/// Bridge `TradeSignalOut` signals to the execution-engine.
///
/// When `EXECUTION_ENGINE_URL` is set (e.g. `http://127.0.0.1:8090`), each
/// signal is POST-ed to `{EXECUTION_ENGINE_URL}/api/signals`.  `TradeSignalOut`
/// and execution-engine's `TradeSignal` share the same JSON schema, so no
/// conversion is needed.
///
/// When the WS path is also active (`ARB_WS_ENABLED=true` and execution-engine
/// has `ARB_WS_URL` set), signals travel via both paths automatically.
fn bridge_signals_to_tokio(signal_rx: Receiver<TradeSignalOut>) {
    let execution_url = std::env::var("EXECUTION_ENGINE_URL").ok();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "bridge thread could not start tokio runtime");
                return;
            }
        };
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        while let Ok(sig) = signal_rx.recv() {
            info!(
                strategy = %sig.strategy,
                edge = sig.expected_edge,
                size_usd = sig.size_usd,
                "trade signal dispatched"
            );
            if let Some(ref base_url) = execution_url {
                let endpoint = format!("{base_url}/api/signals");
                let payload = serde_json::json!({
                    "v": sig.v,
                    "token": sig.token,
                    "direction": sig.direction,
                    "size_usd": sig.size_usd,
                    "confidence": sig.confidence,
                    "expected_edge": sig.expected_edge,
                    "strategy": sig.strategy,
                    "timestamp_ms": sig.timestamp_ms,
                });
                let client = client.clone();
                rt.spawn(async move {
                    if let Err(e) = client.post(&endpoint).json(&payload).send().await {
                        warn!(endpoint = %endpoint, error = %e, "signal POST failed");
                    }
                });
            }
        }
    });
}

#[derive(Clone)]
struct WsState {
    arb_tx: broadcast::Sender<String>,
}

fn spawn_ws_broadcast(arb_rx: Receiver<ArbSignal>) -> WsState {
    let (tx, _) = broadcast::channel(256);
    let ws_tx = tx.clone();
    std::thread::spawn(move || {
        while let Ok(arb) = arb_rx.recv() {
            if let Ok(json) = serde_json::to_string(&arb) {
                let _ = ws_tx.send(json);
            }
        }
    });
    WsState { arb_tx: tx }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<WsState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: WsState) {
    let (mut sender, mut receiver) = socket.split();
    let mut sub = state.arb_tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        loop {
            match sub.recv().await {
                Ok(msg) => {
                    if sender.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(skipped = n, "ws client lagged");
                }
                Err(_) => break,
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
