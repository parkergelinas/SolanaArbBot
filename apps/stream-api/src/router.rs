use axum::{routing::get, Router};
use tower_http::cors::{Any, CorsLayer};

use crate::{routes, AppState};

pub fn build(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(routes::health::health))
        .route("/api/live-signals", get(routes::live_signals::get_live_signals))
        .route("/stream", get(routes::stream::stream_handler))
        .layer(cors)
        .with_state(state)
}
