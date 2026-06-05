use axum::{routing::get, Router};
use tower_http::cors::CorsLayer;

use crate::routes::{health, stream};
use crate::AppState;

pub fn build(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/intelligence", get(stream::stream_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
