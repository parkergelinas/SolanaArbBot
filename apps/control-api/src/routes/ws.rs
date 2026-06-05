//! WebSocket upgrade handler — streams batched `WsBatchFrame` JSON to clients.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};

use crate::{
    dto::SignalEventDto,
    events::WsEvent,
    routes::health::build_health,
    state::AppState,
    stream::WsBatchFrame,
};

/// Upgrade HTTP → WebSocket and hand off to `handle_socket`.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.batch_tx.subscribe();

    // Immediate batch: health + journal replay so reconnect restores trade state.
    let health = build_health(&state, state.deploy_env, "connected").await;
    let mut connect_events = vec![WsEvent::Health(health)];
    for trade in state.recent_trades(AppState::trade_replay_count()).await {
        connect_events.push(WsEvent::Trade(trade));
    }
    let signal_replay = state.signal_store.lock().await;
    let signal_tail: Vec<SignalEventDto> = signal_replay
        .iter()
        .rev()
        .take(50)
        .cloned()
        .collect();
    drop(signal_replay);
    for sig in signal_tail.into_iter().rev() {
        connect_events.push(WsEvent::Signal(sig));
    }

    let connect_batch = WsBatchFrame::new(0, connect_events);

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
