//! WebSocket upgrade handler — streams `WsEvent` JSON to every connected client.
//!
//! On connect, the handler subscribes to the shared broadcast channel and
//! forwards events until the client disconnects or the channel is closed.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};

use crate::{events::WsEvent, state::AppState};

/// Upgrade HTTP → WebSocket and hand off to `handle_socket`.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Drive the lifecycle of a single WebSocket connection.
///
/// Events are read from the broadcast channel and forwarded as JSON text
/// frames.  The loop exits when the client disconnects or the sender is
/// dropped (server shutdown).
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.event_tx.subscribe();

    // Send an immediate health ping so the client knows the stream is live.
    let ping_event = WsEvent::Health(crate::dto::HealthDto {
        status: "connected".to_owned(),
        uptime_secs: state.uptime_secs(),
        signals_stored: state.signal_store.lock().await.len(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    });

    if let Ok(text) = serde_json::to_string(&ping_event) {
        if socket.send(Message::Text(text)).await.is_err() {
            return;
        }
    }

    loop {
        match rx.recv().await {
            Ok(event) => {
                let text = match serde_json::to_string(&event) {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("ws serialisation error: {e}");
                        continue;
                    }
                };
                if socket.send(Message::Text(text)).await.is_err() {
                    // Client disconnected.
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("ws client lagged, dropped {n} events");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}
