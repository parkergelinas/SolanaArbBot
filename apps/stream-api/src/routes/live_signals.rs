use axum::{extract::Query, extract::State, Json};
use serde::Deserialize;
use signal_bus::LiveSignal;

use crate::AppState;

#[derive(Deserialize, Default)]
pub struct LiveSignalQuery {
    pub limit: Option<usize>,
}

/// Recent normalized live signals from the shared signal-bus buffer.
pub async fn get_live_signals(
    State(state): State<AppState>,
    Query(q): Query<LiveSignalQuery>,
) -> Json<Vec<LiveSignal>> {
    let limit = q.limit.unwrap_or(100).min(1_000);
    let signals = state.signal_bus.recent(limit).await;
    Json(signals)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use signal_bus::{from_enriched, whale_to_live_signal, RawWhaleAlert, SignalBus, SignalKind, SOL_MINT};
    use tower::ServiceExt;

    use crate::router::build;
    use crate::AppState;
    use crate::contracts::WSBatchFrame;
    use tokio::sync::broadcast;

    fn sample_state(bus: Arc<SignalBus>) -> AppState {
        let (batch_tx, _) = broadcast::channel::<WSBatchFrame>(8);
        let (_swap_tx, swap_rx) = crossbeam_channel::unbounded();
        let (ws_tx, _ws_rx) = crossbeam_channel::unbounded();
        AppState {
            batch_tx,
            market: Arc::new(crate::market::spawn_market_engine(swap_rx, ws_tx)),
            signal_bus: bus,
            scanners: signals::ScannerStore::new(),
        }
    }

    #[tokio::test]
    async fn live_signals_endpoint_returns_bus_contents() {
        std::env::set_var("SIGNAL_BUFFER_PERSIST", "false");
        let bus = Arc::new(SignalBus::with_defaults());

        let swap = from_enriched(&data_layer::types::EnrichedSwapEvent {
            v: 1,
            swap: data_layer::types::SwapEvent {
                signature: "swap_a".into(),
                wallet: "w1".into(),
                token: SOL_MINT.into(),
                amount_sol: 3.0,
                dex: "raydium".into(),
                timestamp: 100,
            },
            token_symbol: "SOL".into(),
            wallet_label: None,
            notional_usd: 400.0,
            slot: 1,
        });
        let whale = whale_to_live_signal(&RawWhaleAlert {
            alert_id: "whale_a".into(),
            signature: "whale_a".into(),
            wallet: "w2".into(),
            token: "mint".into(),
            token_symbol: "BONK".into(),
            dex: "orca".into(),
            amount_sol: 50.0,
            notional_usd: 8_000.0,
            strength: 0.85,
            confidence: 0.8,
            timestamp: 200,
            detail: None,
        });
        bus.publish(swap).await;
        bus.publish(whale).await;

        let app = build(sample_state(bus));
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
