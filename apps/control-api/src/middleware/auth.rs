//! Bearer / x-api-key auth for control-api REST routes.
//!
//! # Security requirements
//!
//! `CONTROL_API_KEY` **must** be set in production. If it is absent, all
//! non-public API routes return 401 — the bot refuses to operate without an
//! explicit secret rather than silently opening itself to the network.
//!
//! Set `CONTROL_API_INSECURE_SKIP_AUTH=1` **only** in local dev environments
//! where no network exposure is present. Never set this flag in production.

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

/// Returns true when a developer has explicitly opted out of auth (local dev only).
fn insecure_skip_auth() -> bool {
    std::env::var("CONTROL_API_INSECURE_SKIP_AUTH")
        .map(|v| v.trim() == "1")
        .unwrap_or(false)
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

/// Constant-time byte-slice comparison — prevents timing-oracle key recovery.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    // black_box prevents the compiler from short-circuiting the loop.
    std::hint::black_box(diff) == 0
}

fn keys_match(provided: &str, expected: &str) -> bool {
    constant_time_eq(provided.as_bytes(), expected.as_bytes())
}

fn unauthorized(detail: &'static str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({
            "error": "unauthorized",
            "detail": detail
        })),
    )
        .into_response()
}

/// Authenticates all non-public `/api/` routes.
///
/// * Public health/ws paths always pass through.
/// * `CONTROL_API_INSECURE_SKIP_AUTH=1` bypasses auth for local dev only.
/// * If `CONTROL_API_KEY` is **not set**, all protected routes return 401 —
///   production deployments must configure the key explicitly.
/// * Token comparison is constant-time to prevent timing attacks.
pub async fn require_api_key(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_owned();

    if path_is_public(&path) {
        return next.run(req).await;
    }

    if !path.starts_with("/api/") {
        return next.run(req).await;
    }

    // Explicit dev-only opt-out — never set in production.
    if insecure_skip_auth() {
        tracing::warn!(path, "CONTROL_API_INSECURE_SKIP_AUTH is set — skipping auth (dev mode only)");
        return next.run(req).await;
    }

    let expected = match configured_api_key() {
        Some(k) => k,
        None => {
            tracing::error!(
                path,
                "CONTROL_API_KEY is not set; refusing all protected requests. \
                 Set CONTROL_API_KEY or CONTROL_API_INSECURE_SKIP_AUTH=1 for local dev."
            );
            return unauthorized("CONTROL_API_KEY is not configured on this server");
        }
    };

    match extract_token(&req) {
        Some(provided) if keys_match(&provided, &expected) => next.run(req).await,
        _ => unauthorized("missing or invalid CONTROL_API_KEY"),
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

    #[test]
    fn constant_time_eq_correct() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"Secret"));
        assert!(!constant_time_eq(b"a", b"ab"));
    }

    #[test]
    fn keys_match_detects_mismatch() {
        assert!(keys_match("abc123", "abc123"));
        assert!(!keys_match("abc123", "abc124"));
        assert!(!keys_match("abc123", "abc12"));
    }
}
