use axum::{extract::State, Json};

use crate::{
    dto::HealthDto,
    error::ApiResult,
    state::AppState,
};

pub async fn get_health(State(state): State<AppState>) -> ApiResult<Json<HealthDto>> {
    let signal_store = state.signal_store.lock().await;
    Ok(Json(HealthDto {
        status: "ok".to_owned(),
        uptime_secs: state.uptime_secs(),
        signals_stored: signal_store.len(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }))
}
