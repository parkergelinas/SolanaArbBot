use axum::Json;
use serde::Serialize;

use crate::contracts::SCHEMA_VERSION;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub schema_version: u32,
    pub stream_path: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        schema_version: SCHEMA_VERSION,
        stream_path: "/stream",
    })
}
