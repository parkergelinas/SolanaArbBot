//! Forward simulation pass using optimized parameters.

use std::sync::Arc;

use config::SystemConfig;
use serde::{Deserialize, Serialize};

use crate::combined::run_combined_backtest;
use crate::dataset::ReplayDataset;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub net_pnl_usd: f64,
    pub trades_executed: u64,
    pub trades_per_hour: f64,
    pub avg_net_per_trade_usd: f64,
    pub win_rate: f64,
    pub scalp_pnl_usd: f64,
    pub arb_pnl_usd: f64,
    pub duration_hours: f64,
}

/// Runs a forward paper simulation on the tail of the dataset.
pub fn run_forward_simulation(cfg: Arc<SystemConfig>, window: &ReplayDataset) -> SimulationResult {
    let result = run_combined_backtest(cfg, window);
    let m = &result.metrics;

    let executed = m.scalp.winning_trades
        + m.scalp.losing_trades
        + m.arb.winning_trades
        + m.arb.losing_trades;

    let win_rate = if executed == 0 {
        0.0
    } else {
        (m.scalp.winning_trades + m.arb.winning_trades) as f64 / executed as f64
    };

    SimulationResult {
        net_pnl_usd: m.combined_net_pnl_usd,
        trades_executed: executed,
        trades_per_hour: if m.duration_hours > 0.0 {
            executed as f64 / m.duration_hours
        } else {
            0.0
        },
        avg_net_per_trade_usd: if executed == 0 {
            0.0
        } else {
            m.combined_net_pnl_usd / executed as f64
        },
        win_rate,
        scalp_pnl_usd: m.scalp.net_pnl_usd,
        arb_pnl_usd: m.arb.net_pnl_usd,
        duration_hours: m.duration_hours,
    }
}
