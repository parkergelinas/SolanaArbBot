//! Control API — entry point.
//!
//! Starts an axum HTTP server that exposes:
//!   • REST endpoints for system control, config management, signal/portfolio inspection
//!   • A WebSocket endpoint at `/ws` for real-time event streaming
//!
//! Bind address: `CONTROL_API_HOST` (default `0.0.0.0`) + `CONTROL_API_PORT` (default `3001`).
//! **Not suitable for Vercel serverless** — deploy on Railway/Fly/Render/VPS (see docs).

mod bridge;
mod intelligence_bridge;
mod dto;
mod error;
mod events;
mod router;
mod routes;
mod runtime_ctl;
mod runtime_env;
mod signal_bridge;
mod state;
mod store;
mod stream;
mod trade_journal;

use std::process::ExitCode;
use std::time::Duration;

use config::{ConfigError, ConfigHandle};
use data_layer::spawn_pipeline;
use tokio::time;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::{
    bridge::spawn_data_layer_bridge,
    intelligence_bridge::spawn_intelligence_bridge,
    routes::health::build_health,
    runtime_env::{format_startup_errors, validate_for_deploy, RuntimeEnv},
    state::AppState,
};

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(code) = run().await {
        code
    } else {
        ExitCode::SUCCESS
    }
}

async fn run() -> Result<(), ExitCode> {
    // ── Observability ─────────────────────────────────────────────────────────
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // ── Process environment ───────────────────────────────────────────────────
    let runtime = match RuntimeEnv::load() {
        Ok(r) => r,
        Err(errors) => {
            let msg = format_startup_errors("invalid process environment", &errors);
            error!("{msg}");
            eprintln!("{msg}");
            return Err(ExitCode::from(78)); // EX_CONFIG
        }
    };

    // ── Domain configuration ────────────────────────────────────────────────────
    let handle = match ConfigHandle::load() {
        Ok(h) => h,
        Err(ConfigError::Invalid(msg)) => {
            let errors = vec![msg];
            let out = format_startup_errors("invalid system configuration", &errors);
            error!("{out}");
            eprintln!("{out}");
            return Err(ExitCode::from(78));
        }
        Err(e) => {
            error!("config load failed: {e}");
            eprintln!("control-api startup failed — config load: {e}");
            return Err(ExitCode::from(78));
        }
    };

    if let Err(errors) = validate_for_deploy(&runtime, &handle) {
        let out = format_startup_errors("production validation", &errors);
        error!("{out}");
        eprintln!("{out}");
        return Err(ExitCode::from(78));
    }

    if let Some(redis) = store::redis::RedisStoreConfig::from_env() {
        redis.announce_stub();
    }

    let sys_cfg = (*handle).clone();
    info!(
        deploy_env = runtime.deploy_env.as_str(),
        mode = if sys_cfg.features.dry_run { "paper" } else { "live" },
        rpc = %sys_cfg.rpc.primary_endpoint(),
        "SystemConfig loaded — control-api starting"
    );

    // ── Shared state ──────────────────────────────────────────────────────────
    let app_state = AppState::new(sys_cfg, runtime.deploy_env);
    app_state.signal_bus.load_persisted().await;

    // ── Data-layer + intelligence → signal-bus ─────────────────────────────────
    let use_mock = std::env::var("CONTROL_USE_MOCK")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !use_mock {
        match spawn_pipeline() {
            Ok(handles) => {
                let threshold = handles.config.whale_threshold_sol;
                spawn_data_layer_bridge(
                    handles.ws_rx,
                    app_state.signal_bus.clone(),
                    app_state.clone(),
                );
                spawn_intelligence_bridge(
                    handles.whale_rx,
                    threshold,
                    app_state.signal_bus.clone(),
                    app_state.clone(),
                );
                info!("control-api bridged data-layer + intelligence → signal-bus");
            }
            Err(e) => {
                error!("data-layer pipeline failed to start: {e}");
            }
        }
    } else {
        info!("control-api skipping data-layer bridge (CONTROL_USE_MOCK=1)");
    }

    // ── Background: periodic health heartbeat ─────────────────────────────────
    {
        let state = app_state.clone();
        let deploy_env = runtime.deploy_env;
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let health = build_health(&state, deploy_env, "ok").await;
                state.emit(crate::events::WsEvent::Health(health));

                let portfolio =
                    crate::routes::portfolio::build_portfolio_dto(&state).await;
                state.emit(crate::events::WsEvent::Portfolio(portfolio));

                let risk = crate::routes::portfolio::build_risk_dto(&state).await;
                state.emit(crate::events::WsEvent::Risk(risk));
            }
        });
    }

    // ── HTTP server ───────────────────────────────────────────────────────────
    let addr = runtime.listen_addr();
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("failed to bind {addr}: {e}");
            eprintln!("control-api startup failed — cannot bind {addr}: {e}");
            return Err(ExitCode::from(78));
        }
    };

    info!("control-api listening on http://{addr}");
    info!("WebSocket stream at ws://{addr}/ws");
    info!(
        "deploy on Railway/Fly/Render — dashboard proxies REST via CONTROL_API_URL; \
         browser WS uses NEXT_PUBLIC_WS_URL (wss://)"
    );

    let app = router::build(app_state);
    if let Err(e) = axum::serve(listener, app).await {
        error!("server error: {e}");
        return Err(ExitCode::from(1));
    }

    Ok(())
}
