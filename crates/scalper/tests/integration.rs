//! Integration tests for the scalper crate.
//!
//! Each test targets one specific filter or subsystem.  All tests use
//! deterministic synthetic data; no real-time or external dependencies.

use std::sync::Arc;

use common::Pubkey;
use config::{ScalerConfig, SystemConfig};
use scalper::{PaperSimulator, ScalpEngine, TradeCandidate};
use signals::{ComputedFeatures, Direction, FeatureVector, SignalEvent, SignalType};

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

fn make_signal(pool: Pubkey, strength: f64, confidence: f64) -> SignalEvent {
    let now = 1_000_000_000_u64;
    SignalEvent {
        signal_id: 42,
        timestamp_micros: now,
        pool_address: pool,
        signal_type: SignalType::Momentum,
        strength,
        confidence,
        direction: Direction::Long,
        timeframe_secs: 60,
        feature_vector: FeatureVector {
            volume_short: 1_000.0,
            volume_long: 8_000.0,
            price_velocity: 0.5,
            liquidity_delta_pct: 0.0,
            whale_activity_score: 0.3,
            smart_money_score: 0.4,
            data_points: 20,
        },
        explanation: "test signal".to_owned(),
    }
}

/// Build a `ComputedFeatures` with the given pool and market context.
fn make_features(
    pool: Pubkey,
    last_price: f64,
    volume_long: f64,
    price_velocity: f64,
) -> ComputedFeatures {
    ComputedFeatures {
        pool,
        volume_short: volume_long * 0.3,
        volume_long,
        price_velocity,
        liquidity_delta_pct: 0.0,
        whale_activity_score: 0.3,
        smart_money_score: 0.4,
        last_price,
        last_liquidity: 1_000_000.0,
        whale_event_count: 2,
        data_points: 20,
    }
}

/// Build a `SystemConfig` where only `scalper` is customised; all other
/// sub-configs use their safe defaults.
fn make_config(scalper: ScalerConfig) -> Arc<SystemConfig> {
    Arc::new(SystemConfig {
        scalper,
        ..SystemConfig::default()
    })
}

/// A `ScalerConfig` that passes every filter except the one under test.
/// Tests should tighten only the specific field they need.
fn permissive_config() -> ScalerConfig {
    ScalerConfig {
        // Liquidity: very permissive
        min_pool_tvl_usd: 1.0,
        min_volume_5m_usd: 1.0,
        // Volatility: always pass (0.0 ≤ everything ≤ huge)
        volatility_floor: 0.0,
        volatility_ceiling: 1_000.0,
        // Slippage: very permissive
        max_slippage_bps: 100_000.0,
        mev_haircut_bps: 0.0,
        // Rate limits: very permissive
        max_trades_per_hour_per_asset: 10_000,
        max_trades_per_hour_global: 100_000,
        // Cooldown: disabled
        trade_cooldown_secs: 0,
        // Edge: no minimum
        min_edge_bps: f64::NEG_INFINITY,
        // Sizing: produces a valid position
        base_position_pct: 0.02,
        strength_scaling_exponent: 1.5,
        max_position_usd: 5_000.0,
        min_position_usd: 1.0,
        // Signal freshness: very long window so tests never expire
        signal_max_age_secs: 3_600,
        base_fee_bps: 25,
        priority_fee_lamports: 100_000,
    }
}

const NOW: u64 = 1_000_000_000_u64;

// ─────────────────────────────────────────────────────────────────────────────
// 1. LiquidityFilter rejects thin pools
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_liquidity_filter_rejects_thin_pool() {
    let cfg = ScalerConfig {
        min_pool_tvl_usd: 100_000.0,
        min_volume_5m_usd: 5_000.0,
        ..permissive_config()
    };
    let engine = ScalpEngine::new(make_config(cfg));

    let pool = Pubkey::new([1; 32]);
    // pool_tvl_usd = 50_000 — below the 100_000 threshold.
    let result = engine.evaluate(
        make_signal(pool, 0.8, 0.9),
        make_features(pool, 100.0, 10_000.0, 0.5),
        50_000.0, // thin pool
        NOW,
    );
    assert!(result.is_none(), "LiquidityFilter must reject thin-TVL pool");

    // Verify the rejection is attributed correctly in PnL summary.
    let summary = engine.pnl_summary();
    assert_eq!(summary.rejected_trades, 1);
    assert_eq!(summary.total_trades, 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. RateLimitFilter blocks after threshold
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_rate_limit_blocks_after_threshold() {
    let cfg = ScalerConfig {
        max_trades_per_hour_per_asset: 2,
        // Cooldown disabled so only rate-limit is tested.
        trade_cooldown_secs: 0,
        ..permissive_config()
    };
    let engine = ScalpEngine::new(make_config(cfg));

    let pool = Pubkey::new([2; 32]);
    let pool_tvl = 1_000_000.0;
    let features = make_features(pool, 100.0, 50_000.0, 0.5);

    // Use slightly advancing timestamps so the signal is fresh for each call
    // and cooldown (= 0) does not interfere.
    let r1 = engine.evaluate(make_signal(pool, 0.8, 0.9), features.clone(), pool_tvl, NOW);
    let r2 = engine.evaluate(make_signal(pool, 0.8, 0.9), features.clone(), pool_tvl, NOW + 1);
    let r3 = engine.evaluate(make_signal(pool, 0.8, 0.9), features.clone(), pool_tvl, NOW + 2);

    // Signal timestamps must be fresh (within signal_max_age_secs of evaluate-time).
    // The `make_signal` helper sets timestamp_micros = 1_000_000_000 = NOW, so
    // evaluating at NOW, NOW+1, NOW+2 (all ≤ NOW + 3_600s) keeps them fresh.

    assert!(r1.is_some(), "1st trade should succeed");
    assert!(r2.is_some(), "2nd trade should succeed");
    assert!(r3.is_none(), "3rd trade must be blocked by rate limit");

    let summary = engine.pnl_summary();
    assert_eq!(summary.rejected_trades, 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. CooldownFilter blocks rapid re-trade on same pool
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_cooldown_blocks_rapid_retrade() {
    let cfg = ScalerConfig {
        trade_cooldown_secs: 60, // 60-second cooldown
        ..permissive_config()
    };
    let engine = ScalpEngine::new(make_config(cfg));

    let pool = Pubkey::new([3; 32]);
    let pool_tvl = 2_000_000.0;
    let features = make_features(pool, 200.0, 20_000.0, 0.5);

    // First trade succeeds.
    let r1 = engine.evaluate(
        make_signal(pool, 0.9, 0.95),
        features.clone(),
        pool_tvl,
        NOW,
    );
    assert!(r1.is_some(), "first trade should succeed");

    // Second trade at NOW + 10 s — cooldown has 50 s remaining, must be blocked.
    let r2 = engine.evaluate(
        make_signal(pool, 0.9, 0.95),
        features.clone(),
        pool_tvl,
        NOW + 10_000_000, // 10 seconds later in microseconds
    );
    assert!(r2.is_none(), "CooldownFilter must block rapid re-trade");

    // Third trade at NOW + 61 s — cooldown has expired, should succeed.
    let r3 = engine.evaluate(
        make_signal(pool, 0.9, 0.95),
        features.clone(),
        pool_tvl,
        NOW + 61_000_000, // 61 seconds later
    );
    assert!(r3.is_some(), "trade after cooldown expiry should succeed");
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. SlippageFilter rejects oversized trades
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_slippage_filter_rejects_large_trade() {
    // With pool TVL = 100_000 and trade_size ≈ 5_000 (2% of 250_000 capital),
    // round-trip cost ≈ 1_000 bps. A ceiling of 10 bps must reject the trade.
    let cfg = ScalerConfig {
        max_slippage_bps: 10.0, // very tight slippage ceiling
        base_position_pct: 0.02,
        max_position_usd: 50_000.0, // allow large positions
        min_position_usd: 1.0,
        ..permissive_config()
    };
    let system_cfg = SystemConfig {
        scalper: cfg,
        portfolio: {
            let mut p = config::PortfolioConfig::default();
            p.capital_usd = 250_000.0; // large capital → large trade size
            p
        },
        ..SystemConfig::default()
    };
    let engine = ScalpEngine::new(Arc::new(system_cfg));

    let pool = Pubkey::new([4; 32]);
    // Small pool TVL → large price impact.
    let result = engine.evaluate(
        make_signal(pool, 0.9, 0.95),
        make_features(pool, 100.0, 50_000.0, 0.5),
        100_000.0, // thin pool relative to trade size
        NOW,
    );
    assert!(result.is_none(), "SlippageFilter must reject large-impact trade");
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. PaperSimulator applies slippage to fill price
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_paper_simulator_applies_slippage() {
    let sim = PaperSimulator;
    let cfg = ScalerConfig::default();

    let pool = Pubkey::new([5; 32]);
    let entry_price = 100.0;
    let trade_usd = 1_000.0;
    let pool_tvl = 500_000.0;

    let candidate = TradeCandidate {
        signal: make_signal(pool, 0.8, 0.9),
        pool_address: pool,
        pool_tvl_usd: pool_tvl,
        estimated_entry_price: entry_price,
        trade_size_usd: trade_usd,
        estimated_slippage_bps: 4.0,
        estimated_fee_bps: 2.55,
        estimated_round_trip_cost_bps: 13.1,
        net_edge_bps: 8_000.0 - 13.1,
        created_at_micros: NOW,
        expires_at_micros: NOW + 30_000_000,
    };

    let result = sim.simulate_fill(&candidate, pool_tvl, &cfg, NOW);

    assert!(
        result.fill_price > entry_price,
        "Long fill should pay upward slippage: expected > {entry_price}, got {}",
        result.fill_price
    );
    assert!(
        result.actual_slippage_bps > 0.0,
        "actual_slippage_bps must be positive"
    );
    assert!(!result.rejected);
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. PnL attribution is deterministic
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_pnl_attribution_is_deterministic() {
    let cfg = permissive_config();
    let pool = Pubkey::new([6; 32]);
    let pool_tvl = 1_500_000.0;
    let signal = make_signal(pool, 0.75, 0.85);
    let features = make_features(pool, 150.0, 30_000.0, 0.5);

    // Two independent engines with identical config.
    let engine_a = ScalpEngine::new(make_config(cfg.clone()));
    let engine_b = ScalpEngine::new(make_config(cfg));

    let result_a = engine_a
        .evaluate(signal.clone(), features.clone(), pool_tvl, NOW)
        .expect("engine_a should produce a trade");
    let result_b = engine_b
        .evaluate(signal, features, pool_tvl, NOW)
        .expect("engine_b should produce a trade");

    assert_eq!(
        result_a.pnl_bps, result_b.pnl_bps,
        "PnL must be identical for identical inputs"
    );
    assert_eq!(
        result_a.fill_price, result_b.fill_price,
        "fill price must be identical for identical inputs"
    );
    assert_eq!(
        result_a.actual_slippage_bps, result_b.actual_slippage_bps,
        "slippage must be identical for identical inputs"
    );

    let summary_a = engine_a.pnl_summary();
    let summary_b = engine_b.pnl_summary();
    assert_eq!(summary_a.net_pnl_bps, summary_b.net_pnl_bps);
    assert_eq!(summary_a.winning_trades, summary_b.winning_trades);
}
