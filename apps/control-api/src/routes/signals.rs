use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{dto::SignalEventDto, error::ApiResult, state::AppState};

#[derive(Deserialize, Default)]
pub struct SignalQuery {
    /// Maximum number of signals to return (default 100, max 1000).
    pub limit: Option<usize>,
    /// Filter by signal type: "WhaleFlow", "SmartMoney", or "Momentum".
    pub signal_type: Option<String>,
}

/// Returns recent signals from the in-memory ring buffer, newest first.
pub async fn get_signals(
    State(state): State<AppState>,
    Query(q): Query<SignalQuery>,
) -> ApiResult<Json<Vec<SignalEventDto>>> {
    let limit = q.limit.unwrap_or(100).min(1_000);
    let store = state.signal_store.lock().await;

    let signals: Vec<SignalEventDto> = store
        .iter()
        .rev()
        .filter(|s| {
            q.signal_type
                .as_deref()
                .map_or(true, |t| s.signal_type == t)
        })
        .take(limit)
        .cloned()
        .collect();

    Ok(Json(signals))
}
