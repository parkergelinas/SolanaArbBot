//! Combined scalping + DEX-to-DEX backtest runner.

use std::sync::Arc;

use config::SystemConfig;
use serde::{Deserialize, Serialize};

use crate::arb::run_arb_backtest;
use crate::dataset::ReplayDataset;
use crate::metrics::BacktestMetrics;
use crate::scalp::run_scalp_backtest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyKind {
    Scalping,
    DexArb,
    Combined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedBacktestResult {
    pub kind: StrategyKind,
    pub metrics: BacktestMetrics,
}

/// Runs both strategies and merges metrics.
pub fn run_combined_backtest(cfg: Arc<SystemConfig>, dataset: &ReplayDataset) -> CombinedBacktestResult {
    let scalp = run_scalp_backtest(Arc::clone(&cfg), dataset);
    let arb = run_arb_backtest(Arc::clone(&cfg), dataset);

    let metrics = BacktestMetrics {
        scalp: scalp.clone(),
        arb: arb.clone(),
        combined_net_pnl_usd: scalp.net_pnl_usd + arb.net_pnl_usd,
        combined_trades: scalp.winning_trades
            + scalp.losing_trades
            + arb.winning_trades
            + arb.losing_trades,
        combined_trades_per_day: scalp.trades_per_day + arb.trades_per_day,
        duration_hours: dataset.duration_hours(),
    };

    CombinedBacktestResult {
        kind: StrategyKind::Combined,
        metrics,
    }
}
