use axum::{extract::State, Json};

use autonomous::RuntimeSnapshot;
use crate::{error::ApiResult, state::AppState};

pub async fn get_bot_status(State(state): State<AppState>) -> ApiResult<Json<RuntimeSnapshot>> {
    Ok(Json(
        crate::runtime_ctl::current_snapshot(&state).await,
    ))
}
