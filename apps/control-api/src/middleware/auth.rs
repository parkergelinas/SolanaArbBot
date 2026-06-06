//! Bearer / x-api-key auth for control-api REST routes.

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

const API_KEY_HEADER: &str = "x-api-key";

fn configured_api_key() -> Option<String> {
    std::env::var("CONTROL_API_KEY")
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn extract_token(req: &Request) -> Option<String> {
    if let Some(value) = req.headers().get(API_KEY_HEADER) {
        if let Ok(raw) = value.to_str() {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }
    }

    if let Some(value) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(raw) = value.to_str() {
            let trimmed = raw.trim();
            if let Some(token) = trimmed.strip_prefix("Bearer ") {
                let token = token.trim();
                if !token.is_empty() {
                    return Some(token.to_owned());
                }
            }
        }
    }

    None
}

fn path_is_public(path: &str) -> bool {
    path == "/api/health" || path == "/health" || path == "/ws"
}

/// Requires `CONTROL_API_KEY` when configured. Skips `/api/health` and `/ws`.
pub async fn require_api_key(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_owned();

    if path_is_public(&path) {
        return next.run(req).await;
    }

    let Some(expected) = configured_api_key() else {
        return next.run(req).await;
    };

    if !path.starts_with("/api/") {
        return next.run(req).await;
    }

    match extract_token(&req) {
        Some(provided) if provided == expected => next.run(req).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "unauthorized",
                "detail": "missing or invalid CONTROL_API_KEY"
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_paths_skip_auth() {
        assert!(path_is_public("/api/health"));
        assert!(path_is_public("/ws"));
        assert!(!path_is_public("/api/system/start"));
    }
}
