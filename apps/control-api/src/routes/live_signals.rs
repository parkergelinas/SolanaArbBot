use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use signal_bus::LiveSignal;

use crate::{error::ApiResult, state::AppState};

#[derive(Deserialize, Default)]
pub struct LiveSignalQuery {
    pub limit: Option<usize>,
}

/// Returns recent normalized live signals from the shared signal-bus buffer.
pub async fn get_live_signals(
    State(state): State<AppState>,
    Query(q): Query<LiveSignalQuery>,
) -> ApiResult<Json<Vec<LiveSignal>>> {
    let limit = q.limit.unwrap_or(100).min(1_000);
    let signals = state.signal_bus.recent(limit).await;
    Ok(Json(signals))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use config::SystemConfig;
    use signal_bus::{
        from_enriched, whale_to_live_signal, RawWhaleAlert, SignalBus, SignalKind, SOL_MINT,
    };
    use tower::ServiceExt;

    use crate::router::build;
    use crate::runtime_env::DeployEnv;
    use crate::state::AppState;

    #[tokio::test]
    async fn live_signals_endpoint_matches_stream_api_shape() {
        std::env::set_var("SIGNAL_BUFFER_PERSIST", "false");
        let bus = Arc::new(SignalBus::with_defaults());
        let mut state = AppState::new(SystemConfig::default(), DeployEnv::Development);
        state.signal_bus = bus.clone();

        let swap = from_enriched(&data_layer::types::EnrichedSwapEvent {
            v: 1,
            swap: data_layer::types::SwapEvent {
                signature: "swap_b".into(),
                wallet: "w1".into(),
                token: SOL_MINT.into(),
                amount_sol: 2.0,
                dex: "jupiter".into(),
                timestamp: 50,
            },
            token_symbol: "SOL".into(),
            wallet_label: None,
            notional_usd: 300.0,
            slot: 2,
        });
        let whale = whale_to_live_signal(&RawWhaleAlert {
            alert_id: "whale_b".into(),
            signature: "whale_b".into(),
            wallet: "w2".into(),
            token: "mint".into(),
            token_symbol: "SOL".into(),
            dex: "raydium".into(),
            amount_sol: 60.0,
            notional_usd: 9_000.0,
            strength: 0.9,
            confidence: 0.85,
            timestamp: 150,
            detail: None,
        });
        bus.publish(swap).await;
        bus.publish(whale).await;

        let app = build(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/live-signals?limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let signals: Vec<signal_bus::LiveSignal> = serde_json::from_slice(&body).unwrap();
        assert_eq!(signals.len(), 2);
        assert!(signals.iter().any(|s| s.kind == SignalKind::Swap));
        assert!(signals.iter().any(|s| s.kind == SignalKind::WhaleAlert));
        std::env::remove_var("SIGNAL_BUFFER_PERSIST");
    }
}
