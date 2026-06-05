//! Detection latency budget — mock prices, assert cycle < 50ms.

use std::sync::Arc;
use std::time::Instant;

use arb_engine::{
    config::ArbConfig,
    pool_state::PoolStateEngine,
    router::ArbRouter,
    signal::TradeSignalOut,
    types::{unix_ms, PoolPrice},
};
use crossbeam_channel::unbounded;

const SOL: &str = "So11111111111111111111111111111111111111112";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

fn test_config() -> Arc<ArbConfig> {
    Arc::new(ArbConfig {
        min_spread_pct: 0.003,
        min_liquidity_usd: 10_000.0,
        max_stale_ms: 3000,
        min_profit_usd: 0.5,
        min_confidence: 0.5,
        round_trip_fee_pct: 0.003,
        batch_interval_ms: 40,
        mock_interval_ms: 50,
        ws_port: 8091,
        ws_enabled: false,
        max_execution_size_usd: 500.0,
    })
}

#[test]
fn detection_latency_test() {
    let config = test_config();
    let engine = Arc::new(PoolStateEngine::new());
    let now = unix_ms();

    for (dex, price, liq) in [
        ("raydium", 145.0, 50_000.0),
        ("orca", 146.2, 60_000.0),
        ("meteora", 145.5, 45_000.0),
    ] {
        engine.upsert(&PoolPrice {
            dex: dex.into(),
            token_a: SOL.into(),
            token_b: USDC.into(),
            price,
            liquidity: liq,
            timestamp: now,
        });
    }

    let (signal_tx, signal_rx) = unbounded::<TradeSignalOut>();
    let router = ArbRouter::new(config, engine, signal_tx, None);

    let started = Instant::now();
    let emitted = router.run_detection_cycle();
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    assert!(
        elapsed_ms < 50.0,
        "detection cycle took {elapsed_ms:.2}ms, budget is 50ms"
    );
    assert!(emitted >= 1, "expected at least one arb signal");
    assert!(signal_rx.try_recv().is_ok());
}
