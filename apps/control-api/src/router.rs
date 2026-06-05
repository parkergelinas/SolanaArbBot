//! Assembles the axum `Router` from all route modules.

use axum::{routing::get, routing::post, Router};
use tower_http::cors::CorsLayer;

use crate::{routes, state::AppState};

pub fn build(state: AppState) -> Router {
    Router::new()
        // ── Health ────────────────────────────────────────────────────────────
        .route("/api/health", get(routes::health::get_health))
        // ── Configuration ─────────────────────────────────────────────────────
        .route(
            "/api/config",
            get(routes::config::get_config).patch(routes::config::patch_config),
        )
        // ── Signals ───────────────────────────────────────────────────────────
        .route("/api/signals", get(routes::signals::get_signals))
        .route("/api/live-signals", get(routes::live_signals::get_live_signals))
        // ── Portfolio + Risk ──────────────────────────────────────────────────
        .route("/api/portfolio", get(routes::portfolio::get_portfolio))
        .route("/api/risk", get(routes::portfolio::get_risk))
        .route("/api/bot/status", get(routes::bot::get_bot_status))
        // ── System control ────────────────────────────────────────────────────
        .route("/api/system/status", get(routes::system::get_status))
        .route("/api/system/start", post(routes::system::start_system))
        .route("/api/system/stop", post(routes::system::stop_system))
        .route(
            "/api/system/reset-portfolio",
            post(routes::system::reset_portfolio),
        )
        // ── WebSocket stream ──────────────────────────────────────────────────
        .route("/ws", get(routes::ws::ws_handler))
        // ── Middleware ────────────────────────────────────────────────────────
        .layer(CorsLayer::permissive())
        .with_state(state)
}
