use axum::{extract::State, Json};

use crate::{
    dto::{CommandResult, SystemStateDto},
    error::{ApiError, ApiResult},
    events::WsEvent,
    state::AppState,
};

pub async fn get_status(State(state): State<AppState>) -> ApiResult<Json<SystemStateDto>> {
    let sys = state.sys.lock().await;
    Ok(Json(SystemStateDto {
        running: sys.running,
        mode: "paper".to_owned(),
        signals_processed: sys.signals_processed,
        events_processed: sys.events_processed,
        last_signal_ts: sys.last_signal_ts,
    }))
}

pub async fn start_system(
    State(state): State<AppState>,
) -> ApiResult<Json<CommandResult>> {
    {
        let mut sys = state.sys.lock().await;
        if sys.running {
            return Err(ApiError::Conflict(
                "System is already running.".to_owned(),
            ));
        }
        sys.running = true;
    }

    let dto = build_status_dto(&state).await;
    let _ = state.event_tx.send(WsEvent::Status(dto));
    tracing::info!("paper trading started via API");

    Ok(Json(CommandResult::ok(
        "Paper trading engine started (paper mode).",
    )))
}

pub async fn stop_system(
    State(state): State<AppState>,
) -> ApiResult<Json<CommandResult>> {
    {
        let mut sys = state.sys.lock().await;
        if !sys.running {
            return Err(ApiError::Conflict(
                "System is not running.".to_owned(),
            ));
        }
        sys.running = false;
    }

    let dto = build_status_dto(&state).await;
    let _ = state.event_tx.send(WsEvent::Status(dto));
    tracing::info!("paper trading stopped via API");

    Ok(Json(CommandResult::ok("Paper trading engine stopped.")))
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
    SystemStateDto {
        running: sys.running,
        mode: "paper".to_owned(),
        signals_processed: sys.signals_processed,
        events_processed: sys.events_processed,
        last_signal_ts: sys.last_signal_ts,
    }
}
