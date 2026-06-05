use axum::{extract::State, Json};

use crate::{
    dto::{CommandResult, SystemStateDto},
    error::{ApiError, ApiResult},
    events::WsEvent,
    state::AppState,
};

pub async fn get_status(State(state): State<AppState>) -> ApiResult<Json<SystemStateDto>> {
    Ok(Json(build_status_dto(&state).await))
}

pub async fn start_system(
    State(state): State<AppState>,
) -> ApiResult<Json<CommandResult>> {
    {
        let sys = state.sys.lock().await;
        if sys.running {
            return Err(ApiError::Conflict(
                "System is already running.".to_owned(),
            ));
        }
    }

    crate::runtime_ctl::start_runtime(&state)
        .await
        .map_err(ApiError::Internal)?;

    let dto = build_status_dto(&state).await;
    state.emit(WsEvent::Status(dto));
    let (runtime_mode, _, strategies) = crate::runtime_ctl::strategy_status(&state).await;
    tracing::info!(
        runtime_mode = %runtime_mode,
        strategies = ?strategies,
        "autonomous bot started"
    );

    Ok(Json(CommandResult::ok(format!(
        "Autonomous bot started (runtime_mode={runtime_mode})."
    ))))
}

pub async fn stop_system(
    State(state): State<AppState>,
) -> ApiResult<Json<CommandResult>> {
    {
        let sys = state.sys.lock().await;
        if !sys.running {
            return Err(ApiError::Conflict(
                "System is not running.".to_owned(),
            ));
        }
    }

    crate::runtime_ctl::stop_runtime(&state)
        .await
        .map_err(ApiError::Internal)?;

    let dto = build_status_dto(&state).await;
    state.emit(WsEvent::Status(dto));
    tracing::info!("autonomous bot stopped via API");

    Ok(Json(CommandResult::ok("Autonomous bot stopped.")))
}

/// Reset portfolio state (paper mode only).
pub async fn reset_portfolio(
    State(state): State<AppState>,
) -> ApiResult<Json<CommandResult>> {
    {
        let mut sys = state.sys.lock().await;
        sys.signals_processed = 0;
        sys.events_processed = 0;
        sys.last_signal_ts = None;
    }
    state.signal_store.lock().await.clear();

    tracing::info!("portfolio reset via API");
    Ok(Json(CommandResult::ok("Portfolio and signal store reset.")))
}

async fn build_status_dto(state: &AppState) -> SystemStateDto {
    let sys = state.sys.lock().await;
    let (runtime_mode, ingestion_mode, active_strategies) =
        crate::runtime_ctl::strategy_status(state).await;
    SystemStateDto {
        running: sys.running,
        mode: runtime_mode.clone(),
        runtime_mode,
        ingestion_mode,
        active_strategies,
        signals_processed: sys.signals_processed,
        events_processed: sys.events_processed,
        last_signal_ts: sys.last_signal_ts,
    }
}
