//! Verification gates for backtest results.

use serde::{Deserialize, Serialize};

use crate::metrics::BacktestMetrics;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationThresholds {
    pub min_net_pnl_usd: f64,
    pub min_avg_net_per_trade_usd: f64,
    pub min_win_rate: f64,
    pub min_trades_per_day: f64,
    pub max_trades_per_day: f64,
}

impl VerificationThresholds {
    /// Thresholds aligned with user targets ($0.10/trade, 1000 trades/day).
    pub fn for_targets() -> Self {
        Self {
            min_net_pnl_usd: 0.0,
            min_avg_net_per_trade_usd: 0.08,
            min_win_rate: 0.40,
            min_trades_per_day: 200.0,
            max_trades_per_day: 3_000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub passed: bool,
    pub checks: Vec<VerificationCheck>,
}

/// Verifies combined backtest metrics against thresholds.
pub fn verify_metrics(metrics: &BacktestMetrics, thresholds: &VerificationThresholds) -> VerificationReport {
    let executed_scalp = metrics.scalp.winning_trades + metrics.scalp.losing_trades;
    let executed_arb = metrics.arb.winning_trades + metrics.arb.losing_trades;
    let total_executed = executed_scalp + executed_arb;

    let avg_net = if total_executed == 0 {
        0.0
    } else {
        metrics.combined_net_pnl_usd / total_executed as f64
    };

    let combined_win_rate = if total_executed == 0 {
        0.0
    } else {
        (metrics.scalp.winning_trades + metrics.arb.winning_trades) as f64 / total_executed as f64
    };

    let checks = vec![
        VerificationCheck {
            name: "net_pnl_positive".to_owned(),
            passed: metrics.combined_net_pnl_usd >= thresholds.min_net_pnl_usd,
            detail: format!(
                "combined net PnL ${:.2} (min ${:.2})",
                metrics.combined_net_pnl_usd, thresholds.min_net_pnl_usd
            ),
        },
        VerificationCheck {
            name: "avg_net_per_trade".to_owned(),
            passed: avg_net >= thresholds.min_avg_net_per_trade_usd,
            detail: format!(
                "avg ${:.3}/trade (min ${:.2})",
                avg_net, thresholds.min_avg_net_per_trade_usd
            ),
        },
        VerificationCheck {
            name: "win_rate".to_owned(),
            passed: combined_win_rate >= thresholds.min_win_rate,
            detail: format!(
                "win rate {:.1}% (min {:.0}%)",
                combined_win_rate * 100.0,
                thresholds.min_win_rate * 100.0
            ),
        },
        VerificationCheck {
            name: "trades_per_day".to_owned(),
            passed: metrics.combined_trades_per_day >= thresholds.min_trades_per_day
                && metrics.combined_trades_per_day <= thresholds.max_trades_per_day,
            detail: format!(
                "{:.0} trades/day (range {:.0}–{:.0})",
                metrics.combined_trades_per_day,
                thresholds.min_trades_per_day,
                thresholds.max_trades_per_day
            ),
        },
        VerificationCheck {
            name: "scalp_contribution".to_owned(),
            passed: executed_scalp > 0,
            detail: format!("{executed_scalp} scalp trades executed"),
        },
        VerificationCheck {
            name: "arb_contribution".to_owned(),
            passed: executed_arb > 0,
            detail: format!("{executed_arb} arb trades executed"),
        },
    ];

    let passed = checks.iter().all(|c| c.passed);

    VerificationReport { passed, checks }
}
