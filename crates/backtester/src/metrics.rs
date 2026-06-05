//! Backtest performance metrics and aggregation.

use serde::{Deserialize, Serialize};

/// Per-strategy backtest metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyMetrics {
    pub strategy: String,
    pub total_trades: u64,
    pub winning_trades: u64,
    pub losing_trades: u64,
    pub rejected_trades: u64,
    pub gross_pnl_usd: f64,
    pub net_pnl_usd: f64,
    pub total_fees_usd: f64,
    pub win_rate: f64,
    pub avg_net_per_trade_usd: f64,
    pub trades_per_hour: f64,
    pub trades_per_day: f64,
    pub max_drawdown_usd: f64,
    pub sharpe_approx: f64,
}

/// Combined metrics across strategies.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BacktestMetrics {
    pub scalp: StrategyMetrics,
    pub arb: StrategyMetrics,
    pub combined_net_pnl_usd: f64,
    pub combined_trades: u64,
    pub combined_trades_per_day: f64,
    pub duration_hours: f64,
}

impl StrategyMetrics {
    pub fn finalize(&mut self, duration_hours: f64, equity_curve: &[f64]) {
        let executed = self.winning_trades + self.losing_trades;
        self.win_rate = if executed == 0 {
            0.0
        } else {
            self.winning_trades as f64 / executed as f64
        };
        self.avg_net_per_trade_usd = if executed == 0 {
            0.0
        } else {
            self.net_pnl_usd / executed as f64
        };
        self.trades_per_hour = if duration_hours > 0.0 {
            executed as f64 / duration_hours
        } else {
            0.0
        };
        self.trades_per_day = self.trades_per_hour * 24.0;

        self.max_drawdown_usd = compute_max_drawdown(equity_curve);

        self.sharpe_approx = if equity_curve.len() < 2 {
            0.0
        } else {
            let returns: Vec<f64> = equity_curve
                .windows(2)
                .map(|w| w[1] - w[0])
                .collect();
            let mean = returns.iter().sum::<f64>() / returns.len() as f64;
            let var = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
                / returns.len() as f64;
            let std = var.sqrt();
            if std < 1e-9 { 0.0 } else { mean / std * (252.0_f64).sqrt() }
        };
    }
}

fn compute_max_drawdown(equity: &[f64]) -> f64 {
    let mut peak = 0.0_f64;
    let mut max_dd = 0.0_f64;
    for &v in equity {
        peak = peak.max(v);
        max_dd = max_dd.max(peak - v);
    }
    max_dd
}
