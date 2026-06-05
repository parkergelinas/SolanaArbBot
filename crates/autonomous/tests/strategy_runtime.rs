//! Integration tests: strategy controller affects ingestion + trade eligibility.

use autonomous::{
    ExternalIngestionBuffer, IngestionMode, RuntimeMode, StrategyController, TradePolicy,
};
use common::Pubkey;
use config::{StrategyConfig, SystemConfig};
use signals::{Direction, FeatureVector, SignalEvent, SignalType};

fn signal(signal_type: SignalType, price_vel: f64, whale_score: f64) -> SignalEvent {
    SignalEvent {
        signal_id: 1,
        timestamp_micros: 1,
        pool_address: Pubkey::new([2; 32]),
        signal_type,
        strength: 0.65,
        confidence: 0.7,
        direction: Direction::Long,
        timeframe_secs: 60,
        feature_vector: FeatureVector {
            volume_short: 120.0,
            volume_long: 100.0,
            price_velocity: price_vel,
            liquidity_delta_pct: 0.0,
            whale_activity_score: whale_score,
            smart_money_score: whale_score,
            data_points: 12,
        },
        explanation: "test".to_owned(),
    }
}

#[test]
fn whale_copy_vs_momentum_different_eligibility() {
    let whale_policy = TradePolicy {
        trading_enabled: true,
        scalp_enabled: true,
        arb_enabled: true,
        whale_copy: true,
        momentum: false,
        sniper: false,
        dry_run: true,
        min_confidence: 0.05,
        min_strength: 0.05,
    };
    let momentum_policy = TradePolicy {
        momentum: true,
        whale_copy: false,
        ..whale_policy.clone()
    };

    let whale_flow = signal(SignalType::WhaleFlow, 0.0, 0.9);
    let mut whale_only = signal(SignalType::WhaleFlow, 0.0, 0.9);
    whale_only.feature_vector.volume_short = 50.0;
    whale_only.feature_vector.volume_long = 100.0;
    let momentum_sig = signal(SignalType::Momentum, 0.002, 0.0);

    assert!(whale_policy.signal_eligible(&whale_flow));
    assert!(!whale_policy.signal_eligible(&momentum_sig));

    assert!(!momentum_policy.signal_eligible(&whale_only));
    assert!(momentum_policy.signal_eligible(&momentum_sig));
}

#[test]
fn active_mode_drains_external_not_synthetic() {
    let mut ctrl = StrategyController::default();
    ctrl.set_running(true);
    ctrl.apply_selection(StrategyConfig {
        whale_copy: true,
        momentum: false,
        scalp: true,
        arb: false,
        sniper: false,
    });

    let cfg = SystemConfig::default();
    let plan = ctrl.plan_cycle(&cfg);
    assert_eq!(plan.runtime_mode, RuntimeMode::Active);
    assert_eq!(plan.ingestion.mode, IngestionMode::Intelligence);
    assert!(!plan.ingestion.inject_synthetic_whales);

    let mut external = ExternalIngestionBuffer::default();
    assert!(external.drain_market().is_empty());
    assert!(external.drain_whales().is_empty());
}

#[test]
fn paper_mode_uses_synthetic_ingestion() {
    let mut ctrl = StrategyController::default();
    ctrl.set_running(true);
    let plan = ctrl.plan_cycle(&SystemConfig::default());
    assert_eq!(plan.runtime_mode, RuntimeMode::Paper);
    assert_eq!(plan.ingestion.mode, IngestionMode::Synthetic);
    assert!(plan.ingestion.inject_synthetic_whales);
}
