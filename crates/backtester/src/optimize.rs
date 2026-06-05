//! Grid-search parameter optimization for dual-strategy backtesting.

use std::sync::Arc;

use config::SystemConfig;
use serde::{Deserialize, Serialize};

use crate::combined::run_combined_backtest;
use crate::dataset::ReplayDataset;
use crate::verify::{VerificationThresholds, verify_metrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedParams {
    pub min_edge_bps: f64,
    pub trade_cooldown_secs: u64,
    pub min_profit_threshold_usd: f64,
    pub simulation_initial_amount_usd: f64,
    pub min_position_usd: f64,
    pub max_trades_per_hour_global: u32,
    pub signal_cooldown_secs: u64,
}

impl OptimizedParams {
    pub fn apply_to(&self, cfg: &mut SystemConfig) {
        cfg.scalper.min_edge_bps = self.min_edge_bps;
        cfg.scalper.trade_cooldown_secs = self.trade_cooldown_secs;
        cfg.scalper.min_position_usd = self.min_position_usd;
        cfg.scalper.max_trades_per_hour_global = self.max_trades_per_hour_global;
        cfg.execution.min_profit_threshold_usd = self.min_profit_threshold_usd;
        cfg.execution.simulation_initial_amount_usd = self.simulation_initial_amount_usd;
        cfg.signal_engine.cooldown_secs = self.signal_cooldown_secs;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationCandidate {
    pub params: OptimizedParams,
    pub score: f64,
    pub net_pnl_usd: f64,
    pub trades_per_day: f64,
    pub avg_net_per_trade: f64,
    pub verification_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub best: OptimizedParams,
    pub best_score: f64,
    pub candidates_evaluated: u32,
    pub top_candidates: Vec<OptimizationCandidate>,
}

fn score_result(
    net_pnl: f64,
    trades_per_day: f64,
    avg_net: f64,
    verification_passed: bool,
    thresholds: &VerificationThresholds,
) -> f64 {
    let mut score = net_pnl;

    if avg_net >= thresholds.min_avg_net_per_trade_usd {
        score += avg_net * 50.0;
    } else {
        score -= 10.0;
    }

    if trades_per_day >= thresholds.min_trades_per_day {
        score += (trades_per_day / 1000.0) * 5.0;
    } else {
        score -= (thresholds.min_trades_per_day - trades_per_day) * 0.01;
    }

    if verification_passed {
        score += 20.0;
    }

    score
}

/// Grid-searches strategy parameters on the training dataset.
pub fn optimize_params(
    base_cfg: Arc<SystemConfig>,
    dataset: &ReplayDataset,
    thresholds: &VerificationThresholds,
) -> OptimizationResult {
    let min_edge_options = [25.0, 30.0, 35.0];
    let cooldown_options = [15_u64, 20, 25];
    let profit_options = [0.08, 0.10, 0.12];
    let size_options = [200.0, 250.0];
    let signal_cooldown_options = [3_u64, 5];

    let mut candidates: Vec<OptimizationCandidate> = Vec::new();
    let mut evaluated = 0u32;

    for &min_edge in &min_edge_options {
        for &cooldown in &cooldown_options {
            for &min_profit in &profit_options {
                for &size in &size_options {
                    for &sig_cd in &signal_cooldown_options {
                        evaluated += 1;
                        let mut cfg = (*base_cfg).clone();
                        let params = OptimizedParams {
                            min_edge_bps: min_edge,
                            trade_cooldown_secs: cooldown,
                            min_profit_threshold_usd: min_profit,
                            simulation_initial_amount_usd: size,
                            min_position_usd: size,
                            max_trades_per_hour_global: 120,
                            signal_cooldown_secs: sig_cd,
                        };
                        params.apply_to(&mut cfg);

                        let result = run_combined_backtest(Arc::new(cfg), dataset);
                        let verification = verify_metrics(&result.metrics, thresholds);

                        let executed = result.metrics.scalp.winning_trades
                            + result.metrics.scalp.losing_trades
                            + result.metrics.arb.winning_trades
                            + result.metrics.arb.losing_trades;
                        let avg_net = if executed == 0 {
                            0.0
                        } else {
                            result.metrics.combined_net_pnl_usd / executed as f64
                        };

                        let score = score_result(
                            result.metrics.combined_net_pnl_usd,
                            result.metrics.combined_trades_per_day,
                            avg_net,
                            verification.passed,
                            thresholds,
                        );

                        candidates.push(OptimizationCandidate {
                            params,
                            score,
                            net_pnl_usd: result.metrics.combined_net_pnl_usd,
                            trades_per_day: result.metrics.combined_trades_per_day,
                            avg_net_per_trade: avg_net,
                            verification_passed: verification.passed,
                        });
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let best = candidates.first().cloned().unwrap_or_else(|| OptimizationCandidate {
        params: OptimizedParams {
            min_edge_bps: 35.0,
            trade_cooldown_secs: 20,
            min_profit_threshold_usd: 0.10,
            simulation_initial_amount_usd: 200.0,
            min_position_usd: 200.0,
            max_trades_per_hour_global: 100,
            signal_cooldown_secs: 5,
        },
        score: 0.0,
        net_pnl_usd: 0.0,
        trades_per_day: 0.0,
        avg_net_per_trade: 0.0,
        verification_passed: false,
    });

    OptimizationResult {
        best: best.params,
        best_score: best.score,
        candidates_evaluated: evaluated,
        top_candidates: candidates.into_iter().take(10).collect(),
    }
}
