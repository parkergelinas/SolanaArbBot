use axum::{extract::State, Json};
use serde_json::Value;

use config::SystemConfig;

use crate::{
    dto::CommandResult,
    error::{ApiError, ApiResult},
    events::WsEvent,
    state::AppState,
};

/// Returns the full current SystemConfig as JSON.
pub async fn get_config(State(state): State<AppState>) -> ApiResult<Json<SystemConfig>> {
    let cfg = state.config.read().await;
    Ok(Json(cfg.clone()))
}

/// Accepts a partial JSON object and merges it into the current config.
///
/// The merged result is validated before being applied.
/// On success, a `ConfigChanged` WebSocket event is broadcast.
pub async fn patch_config(
    State(state): State<AppState>,
    Json(patch): Json<Value>,
) -> ApiResult<Json<CommandResult>> {
    // Serialise current config to Value, merge the patch, deserialise back.
    let current = {
        let cfg = state.config.read().await;
        serde_json::to_value(cfg.clone()).map_err(|e| ApiError::Internal(e.to_string()))?
    };

    let merged = deep_merge(current, patch);

    let new_cfg: SystemConfig = serde_json::from_value(merged)
        .map_err(|e| ApiError::BadRequest(format!("invalid config shape: {e}")))?;

    new_cfg
        .validate()
        .map_err(|e| ApiError::BadRequest(format!("validation failed: {e}")))?;

    *state.config.write().await = new_cfg;

    state.emit(WsEvent::ConfigChanged {
        section: "system".to_owned(),
        summary: "configuration updated via API".to_owned(),
    });

    tracing::info!("SystemConfig updated via PATCH /api/config");
    Ok(Json(CommandResult::ok("Configuration updated and validated.")))
}

/// Deep-merges `patch` values into `base`.
///
/// For object nodes, keys from `patch` override keys in `base` recursively.
/// Scalar/array values in `patch` replace those in `base`.
fn deep_merge(base: Value, patch: Value) -> Value {
    match (base, patch) {
        (Value::Object(mut base_map), Value::Object(patch_map)) => {
            for (k, v) in patch_map {
                let merged = if let Some(base_v) = base_map.remove(&k) {
                    deep_merge(base_v, v)
                } else {
                    v
                };
                base_map.insert(k, merged);
            }
            Value::Object(base_map)
        }
        (_base, patch) => patch,
    }
}
