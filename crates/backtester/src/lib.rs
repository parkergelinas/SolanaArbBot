//! Historical replay backtesting for scalping and DEX-to-DEX arbitrage strategies.
//!
//! # Pipeline
//!
//! ```text
//! generate dataset → baseline backtest → verify → optimize → retest → simulate
//! ```

#![forbid(unsafe_code)]

pub mod arb;
pub mod combined;
pub mod dataset;
pub mod metrics;
pub mod optimize;
pub mod quote_arb;
pub mod rankings;
pub mod scalp;
pub mod simulate;
pub mod verify;

pub use combined::{run_combined_backtest, CombinedBacktestResult, StrategyKind};
pub use dataset::{generate_dataset, ReplayDataset, ReplayEvent};
pub use metrics::{BacktestMetrics, StrategyMetrics};
pub use optimize::{optimize_params, OptimizedParams, OptimizationResult};
pub use quote_arb::run_quote_arb_backtest;
pub use rankings::{build_strategy_rankings, StrategyRanking};
pub use simulate::{run_forward_simulation, SimulationResult};
pub use verify::{verify_metrics, VerificationReport, VerificationThresholds};

use std::sync::Arc;

use config::SystemConfig;
use serde::{Deserialize, Serialize};

/// Default strategy config tuned for $0.10–$0.50/trade micro-profit targets.
pub fn strategy_config() -> SystemConfig {
    let mut cfg = SystemConfig::default();
    cfg.portfolio.capital_usd = 10_000.0;
    cfg.scalper.min_edge_bps = 30.0;
    cfg.scalper.trade_cooldown_secs = 20;
    cfg.scalper.min_position_usd = 200.0;
    cfg.scalper.max_position_usd = 350.0;
    cfg.scalper.max_trades_per_hour_global = 120;
    cfg.scalper.max_trades_per_hour_per_asset = 30;
    cfg.scalper.min_pool_tvl_usd = 50_000.0;
    cfg.scalper.min_volume_5m_usd = 50.0;
    cfg.scalper.mev_haircut_bps = 5.0;
    cfg.scalper.volatility_floor = 0.0;
    cfg.scalper.volatility_ceiling = 10.0;
    cfg.scalper.max_slippage_bps = 150.0;
    cfg.scalper.min_edge_bps = 8.0;
    cfg.scalper.trade_cooldown_secs = 5;
    cfg.signal_engine.signal_min_strength = 0.05;
    cfg.signal_engine.signal_min_confidence = 0.05;
    cfg.signal_engine.cooldown_secs = 5;
    cfg.signal_engine.signal_channel_capacity = 65_536;
    cfg.execution.min_profit_threshold_usd = 0.10;
    cfg.execution.simulation_initial_amount_usd = 200.0;
    cfg.execution.max_trade_size_usd = 350.0;
    cfg
}

/// Full backtest pipeline output written to disk and consumed by the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineReport {
    pub generated_at: String,
    pub baseline: CombinedBacktestResult,
    pub baseline_verification: VerificationReport,
    pub optimization: OptimizationResult,
    pub retest: CombinedBacktestResult,
    pub retest_verification: VerificationReport,
    pub simulation: SimulationResult,
    pub quote_arb_holdout: StrategyMetrics,
    pub strategy_rankings: Vec<StrategyRanking>,
    pub recommended_config: SystemConfig,
}

/// Runs the complete backtest pipeline on a synthetic dataset.
pub fn run_pipeline(
    cfg: Arc<SystemConfig>,
    dataset: &ReplayDataset,
    train_fraction: f64,
) -> PipelineReport {
    let split_idx = (dataset.events.len() as f64 * train_fraction) as usize;
    let train = dataset.slice(0, split_idx);
    let holdout = dataset.slice(split_idx, dataset.events.len());

    let baseline = run_combined_backtest(Arc::clone(&cfg), &train);

    let thresholds = VerificationThresholds::for_targets();
    let baseline_verification = verify_metrics(&baseline.metrics, &thresholds);

    let optimization = optimize_params(Arc::clone(&cfg), &train, &thresholds);

    let mut optimized_cfg = (*cfg).clone();
    optimization.best.apply_to(&mut optimized_cfg);
    let optimized_cfg = Arc::new(optimized_cfg);

    let retest = run_combined_backtest(Arc::clone(&optimized_cfg), &holdout);
    let retest_verification = verify_metrics(&retest.metrics, &thresholds);

    let sim_window = holdout.slice(
        holdout.events.len().saturating_sub(holdout.events.len() / 3),
        holdout.events.len(),
    );
    let simulation = run_forward_simulation(optimized_cfg.clone(), &sim_window);

    let quote_arb_holdout = run_quote_arb_backtest(Arc::clone(&optimized_cfg), &holdout);
    let strategy_rankings = build_strategy_rankings(
        &retest.metrics.scalp,
        &retest.metrics.arb,
        &quote_arb_holdout,
        retest.metrics.combined_net_pnl_usd,
        retest.metrics.combined_trades_per_day,
        &retest_verification,
    );

    let generated_at = chrono_lite_timestamp();

    PipelineReport {
        generated_at,
        baseline,
        baseline_verification,
        optimization,
        retest,
        retest_verification,
        simulation,
        quote_arb_holdout,
        strategy_rankings,
        recommended_config: Arc::try_unwrap(optimized_cfg).unwrap_or_else(|arc| (*arc).clone()),
    }
}

fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
