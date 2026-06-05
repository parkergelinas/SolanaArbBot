use std::collections::HashMap;

use axum::{extract::State, Json};
use config::SystemConfig;

use crate::{
    dto::HealthDto,
    error::ApiResult,
    runtime_env::DeployEnv,
    state::AppState,
};

/// Build a health snapshot shared by REST `/api/health` and WebSocket connect batch.
pub async fn build_health(state: &AppState, deploy_env: DeployEnv, status: &str) -> HealthDto {
    let signal_store = state.signal_store.lock().await;
    let sys = state.sys.lock().await;
    let cfg = state.config.read().await;

    let mode = trading_mode(&cfg);
    let mut checks = readiness_checks(&cfg, deploy_env);

    if sys.running {
        checks.insert("bot".to_owned(), "ok".to_owned());
    } else {
        checks.insert("bot".to_owned(), "idle".to_owned());
    }

    HealthDto {
        status: status.to_owned(),
        uptime_secs: state.uptime_secs(),
        signals_stored: signal_store.len(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        bot_running: sys.running,
        deploy_env: deploy_env.as_str().to_owned(),
        mode,
        events_processed: state.events_counter.load(std::sync::atomic::Ordering::Relaxed),
        checks,
    }
}

fn trading_mode(cfg: &SystemConfig) -> String {
    if cfg.features.enable_live_trading && !cfg.features.dry_run {
        "live".to_owned()
    } else {
        "paper".to_owned()
    }
}

fn readiness_checks(cfg: &SystemConfig, deploy_env: DeployEnv) -> HashMap<String, String> {
    let mut checks = HashMap::new();

    checks.insert(
        "rpc".to_owned(),
        if cfg.rpc.endpoints.is_empty() {
            "missing".to_owned()
        } else {
            "ok".to_owned()
        },
    );

    checks.insert(
        "websocket".to_owned(),
        if cfg.websocket.endpoints.is_empty() {
            "missing".to_owned()
        } else {
            "ok".to_owned()
        },
    );

    if deploy_env.is_strict() {
        let custom_rpc = std::env::var("SOLANA_ARB_RPC__ENDPOINTS").is_ok();
        checks.insert(
            "rpc_production".to_owned(),
            if custom_rpc {
                "ok".to_owned()
            } else {
                "degraded".to_owned()
            },
        );
    }

    if cfg.features.enable_live_trading && !cfg.features.dry_run {
        let key_ok = cfg.wallet.keypair_env_var.as_ref().is_some_and(|var| {
            std::env::var(var).map(|v| !v.trim().is_empty()).unwrap_or(false)
        }) || cfg
            .wallet
            .keypair_path
            .as_ref()
            .is_some_and(|p| std::path::Path::new(p).exists());
        checks.insert(
            "keypair".to_owned(),
            if key_ok {
                "ok".to_owned()
            } else {
                "missing".to_owned()
            },
        );
    }

    checks
}

pub async fn get_health(State(state): State<AppState>) -> ApiResult<Json<HealthDto>> {
    let deploy_env = state.deploy_env;
    Ok(Json(build_health(&state, deploy_env, "ok").await))
}
