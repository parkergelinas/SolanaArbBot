//! End-to-end backtest pipeline integration test.

use std::sync::Arc;

use backtester::{
    generate_dataset, run_combined_backtest, run_pipeline, strategy_config, verify_metrics,
    VerificationThresholds,
};

#[test]
fn combined_backtest_produces_trades() {
    let cfg = Arc::new(strategy_config());
    let dataset = generate_dataset(3600, 8);
    let result = run_combined_backtest(cfg, &dataset);

    let executed = result.metrics.scalp.winning_trades
        + result.metrics.scalp.losing_trades
        + result.metrics.arb.winning_trades
        + result.metrics.arb.losing_trades;

    assert!(dataset.events.len() > 50, "dataset should have events");
    assert!(executed > 0, "expected at least one executed trade");
}

#[test]
fn full_pipeline_runs_without_panic() {
    let cfg = Arc::new(strategy_config());
    let dataset = generate_dataset(1200, 15);
    let report = run_pipeline(cfg, &dataset, 0.80);

    assert!(report.optimization.candidates_evaluated > 0);
    assert!(report.simulation.duration_hours > 0.0);
}

#[test]
fn verification_checks_run() {
    let cfg = Arc::new(strategy_config());
    let dataset = generate_dataset(3600, 8);
    let result = run_combined_backtest(cfg, &dataset);
    let report = verify_metrics(&result.metrics, &VerificationThresholds::for_targets());

    assert!(!report.checks.is_empty());
}
