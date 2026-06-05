//! Shared live signal buffer for control-api and stream consumers.
//!
//! Parsing lives in [`adapter`]; dedup / storage in [`buffer`]; strategy tags in [`classify`].

#![forbid(unsafe_code)]

pub mod adapter;
pub mod buffer;
pub mod bus;
pub mod classify;
pub mod persist;
pub mod types;

pub use adapter::{
    engine_to_live_signal, from_enriched, from_swap, parse_batch_frame, smart_money_to_live_signal,
    whale_to_live_signal, EngineSignalInput, RawSmartMoneyAlert, RawWhaleAlert, SOL_MINT, USDC_MINT,
};
/// Back-compat alias for bridge modules.
pub use from_enriched as enriched_to_live_signal;
pub use buffer::{BufferConfig, SignalBuffer, DEFAULT_CAPACITY, DEFAULT_DEDUP_TTL_MS};
pub use bus::SignalBus;
pub use classify::strategy_tag_for;
pub use types::{
    alert_id_hash, AlertType, LiveSignal, SignalKind, SignalSourceMeta, StrategyTag, SCHEMA_VERSION,
};

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn swap_and_whale_share_same_buffer() {
        std::env::set_var("SIGNAL_BUFFER_PERSIST", "false");
        let bus = SignalBus::with_defaults();

        let swap = from_enriched(&data_layer::types::EnrichedSwapEvent {
            v: 1,
            swap: data_layer::types::SwapEvent {
                signature: "swap_sig".into(),
                wallet: "w1".into(),
                token: SOL_MINT.into(),
                amount_sol: 5.0,
                dex: "raydium".into(),
                timestamp: 1000,
            },
            token_symbol: "SOL".into(),
            wallet_label: None,
            notional_usd: 500.0,
            slot: 10,
        });

        let whale = whale_to_live_signal(&RawWhaleAlert {
            alert_id: "whale_1".into(),
            signature: "whale_sig".into(),
            wallet: "w2".into(),
            token: "mint".into(),
            token_symbol: "BONK".into(),
            dex: "orca".into(),
            amount_sol: 80.0,
            notional_usd: 12_000.0,
            strength: 0.9,
            confidence: 0.88,
            timestamp: 2000,
            detail: None,
        });

        assert!(bus.publish(swap).await.is_some());
        assert!(bus.publish(whale).await.is_some());

        let recent = bus.recent(10).await;
        assert_eq!(recent.len(), 2);
        assert!(recent.iter().any(|s| s.kind == SignalKind::Swap));
        assert!(recent.iter().any(|s| s.kind == SignalKind::WhaleAlert));

        let mut sub = bus.subscribe();
        let replay = bus.replay().await;
        assert_eq!(replay.len(), 2);

        // Subscriber receives on next publish
        let extra = whale_to_live_signal(&RawWhaleAlert {
            alert_id: "whale_2".into(),
            signature: "whale_sig_2".into(),
            wallet: "w3".into(),
            token: "mint2".into(),
            token_symbol: "SOL".into(),
            dex: "jupiter".into(),
            amount_sol: 20.0,
            notional_usd: 4_000.0,
            strength: 0.7,
            confidence: 0.75,
            timestamp: 3000,
            detail: None,
        });
        bus.publish(extra).await;
        assert!(sub.try_recv().is_ok());

        std::env::remove_var("SIGNAL_BUFFER_PERSIST");
    }

    /// Simulates control-api + stream-api reading the same `Arc<SignalBus>`.
    #[tokio::test]
    async fn shared_bus_serves_both_api_consumers() {
        std::env::set_var("SIGNAL_BUFFER_PERSIST", "false");
        let bus = std::sync::Arc::new(SignalBus::with_defaults());

        bus.publish(from_enriched(&data_layer::types::EnrichedSwapEvent {
            v: 1,
            swap: data_layer::types::SwapEvent {
                signature: "shared_swap".into(),
                wallet: "w".into(),
                token: SOL_MINT.into(),
                amount_sol: 1.0,
                dex: "raydium".into(),
                timestamp: 1,
            },
            token_symbol: "SOL".into(),
            wallet_label: None,
            notional_usd: 150.0,
            slot: 1,
        }))
        .await;
        bus.publish(whale_to_live_signal(&RawWhaleAlert {
            alert_id: "shared_whale".into(),
            signature: "shared_whale".into(),
            wallet: "w".into(),
            token: "m".into(),
            token_symbol: "BONK".into(),
            dex: "orca".into(),
            amount_sol: 40.0,
            notional_usd: 6_000.0,
            strength: 0.8,
            confidence: 0.75,
            timestamp: 2,
            detail: None,
        }))
        .await;

        let control_view = bus.recent(50).await;
        let stream_view = bus.recent(50).await;
        assert_eq!(control_view, stream_view);
        assert_eq!(control_view.len(), 2);

        std::env::remove_var("SIGNAL_BUFFER_PERSIST");
    }
}
