use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use signal_bus::SignalBus;
use tokio::sync::broadcast;

use crate::bridge::live_to_stream_swap;
use crate::contracts::{WSBatchFrame, WSMessage, SCHEMA_VERSION};
use crate::AppState;

pub async fn stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state.batch_tx, state.signal_bus))
}

async fn handle_socket(
    mut socket: WebSocket,
    batch_tx: broadcast::Sender<WSBatchFrame>,
    signal_bus: Arc<SignalBus>,
) {
    let mut rx = batch_tx.subscribe();

    let replay: Vec<WSMessage> = signal_bus
        .replay()
        .await
        .into_iter()
        .map(|live| WSMessage::Swap(live_to_stream_swap(&live)))
        .collect();

    let mut hello_messages = replay;
    hello_messages.push(WSMessage::Signal(crate::contracts::Signal {
        v: SCHEMA_VERSION,
        signal_id: "stream_ready".to_owned(),
        mint: "system".to_owned(),
        kind: crate::contracts::SignalKind::Momentum,
        strength: 1.0,
        confidence: 1.0,
        timestamp_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        detail: Some("connected".to_owned()),
    }));

    let hello = WSBatchFrame::new(0, hello_messages);

    if let Ok(text) = serde_json::to_string(&hello) {
        if socket.send(Message::Text(text)).await.is_err() {
            return;
        }
    }

    loop {
        match rx.recv().await {
            Ok(frame) => {
                let text = match serde_json::to_string(&frame) {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("ws serialize error: {e}");
                        continue;
                    }
                };
                if socket.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("ws client lagged, dropped {n} batches");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}
