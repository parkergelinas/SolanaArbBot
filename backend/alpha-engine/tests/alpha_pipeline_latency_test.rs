//! Alpha pipeline latency — mock inputs, assert cycle < 50ms.

use std::sync::Arc;
use std::time::Instant;

use alpha_engine::{
    config::AlphaConfig,
    emitter::SignalEmitter,
    fusion::FusionEngine,
    pipeline::AlphaPipeline,
    types::{ArbSignal, LiquiditySnapshot, MarketSignal, WalletScore, unix_ms},
};
use tokio::sync::mpsc;

const SOL: &str = "So11111111111111111111111111111111111111112";

fn test_config() -> Arc<AlphaConfig> {
    Arc::new(AlphaConfig {
        feature_ttl_ms: 5000,
        wallet_idle_decay_ms: 30_000,
        min_liquidity_usd: 10_000.0,
        volume_spike_threshold: 2.0,
        weights: alpha_engine::types::ScoringWeights {
            w1: 0.30,
            w2: 0.20,
            w3: 0.25,
            w4: 0.15,
            w5: 0.10,
        },
        min_alpha_score: 0.5,
        min_confidence: 0.5,
        min_liquidity: 0.3,
        cooldown_secs: 0,
        queue_max: 100,
        queue_ttl_ms: 5000,
        drain_per_tick: 5,
        tick_interval_ms: 40,
        base_size_usd: 100.0,
        max_position_usd: 500.0,
        arb_ws_url: None,
        mock_mode: false,
    })
}

#[test]
fn alpha_pipeline_latency_test() {
    let config = test_config();
    let fusion = Arc::new(FusionEngine::new(config.clone()));
    let (tx, _rx) = mpsc::unbounded_channel();
    let emitter = SignalEmitter::new(tx);
    let pipeline = AlphaPipeline::new(config, fusion.clone(), emitter);

    let ts = unix_ms();
    fusion.on_wallet(&WalletScore {
        wallet: "whale_1".into(),
        score: 0.95,
        confidence: 0.9,
        token: SOL.into(),
        timestamp: ts,
    });
    fusion.on_market(&MarketSignal {
        token: SOL.into(),
        momentum: 0.8,
        volume_spike: 2.5,
        price_change: 0.04,
        timestamp: ts,
    });
    fusion.on_liquidity(&LiquiditySnapshot {
        token: SOL.into(),
        liquidity_usd: 80_000.0,
        spread_bps: 20.0,
        timestamp: ts,
    });
    fusion.on_arb(&ArbSignal {
        token_pair: format!("{SOL}/USDC"),
        spread_pct: 0.008,
        confidence: 0.9,
    });

    let started = Instant::now();
    pipeline.on_token_updated(SOL, "whale_1");
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    assert!(
        elapsed_ms < 50.0,
        "alpha pipeline took {elapsed_ms:.2}ms, budget is 50ms"
    );
}
