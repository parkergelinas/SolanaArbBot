//! Control API — entry point.
//!
//! Starts an axum HTTP server that exposes:
//!   • REST endpoints for system control, config management, signal/portfolio inspection
//!   • A WebSocket endpoint at `/ws` for real-time event streaming
//!
//! The server binds to `0.0.0.0:3001` by default (overridable via `CONTROL_API_PORT`).

mod dto;
mod error;
mod events;
mod router;
mod routes;
mod state;
mod stream;

use std::time::Duration;

use config::ConfigHandle;
use tokio::time;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::{
    dto::HealthDto,
    events::WsEvent,
    routes::portfolio::{build_portfolio_dto, build_risk_dto},
    state::AppState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Observability ─────────────────────────────────────────────────────────
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // ── Configuration ─────────────────────────────────────────────────────────
    let handle = ConfigHandle::load()?;
    let sys_cfg = (*handle).clone();
    info!("SystemConfig loaded — control-api starting");

    // ── Shared state ──────────────────────────────────────────────────────────
    let app_state = AppState::new(sys_cfg);

    // ── Background: periodic health heartbeat ─────────────────────────────────
    {
        let state = app_state.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let signals_stored = state.signal_store.lock().await.len();
                let health = HealthDto {
                    status: "ok".to_owned(),
                    uptime_secs: state.uptime_secs(),
                    signals_stored,
                    version: env!("CARGO_PKG_VERSION").to_owned(),
                };
                state.emit(WsEvent::Health(health));

                let portfolio = build_portfolio_dto(&state).await;
                state.emit(WsEvent::Portfolio(portfolio));

                let risk = build_risk_dto(&state).await;
                state.emit(WsEvent::Risk(risk));
            }
        });
    }

    // ── HTTP server ───────────────────────────────────────────────────────────
    let port: u16 = std::env::var("CONTROL_API_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3001);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("control-api listening on http://{addr}");
    info!("WebSocket stream at  ws://{addr}/ws");

    let app = router::build(app_state);
    axum::serve(listener, app).await?;

    Ok(())
}
