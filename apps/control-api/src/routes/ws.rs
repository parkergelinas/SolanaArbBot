//! WebSocket upgrade handler — streams batched `WsBatchFrame` JSON to clients.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};

use crate::{dto::HealthDto, events::WsEvent, state::AppState, stream::WsBatchFrame};

/// Upgrade HTTP → WebSocket and hand off to `handle_socket`.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.batch_tx.subscribe();

    // Immediate batch so the client knows the stream is live.
    let connect_batch = WsBatchFrame::new(
        0,
        vec![WsEvent::Health(HealthDto {
            status: "connected".to_owned(),
            uptime_secs: state.uptime_secs(),
            signals_stored: state.signal_store.lock().await.len(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        })],
    );

    if let Ok(text) = serde_json::to_string(&connect_batch) {
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
                        tracing::warn!("ws serialisation error: {e}");
                        continue;
                    }
                };
                if socket.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("ws client lagged, dropped {n} batch frames");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}
