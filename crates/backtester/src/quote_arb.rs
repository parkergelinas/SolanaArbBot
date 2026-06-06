//! Route-divergence / quote-arb synthetic backtest.
//!
//! Models restricted-route haircut vs unrestricted route on cross-DEX reserve skew.

use std::sync::Arc;

use config::SystemConfig;

use crate::dataset::{ReplayDataset, ReplayEvent};
use crate::metrics::StrategyMetrics;
use common::MarketEvent;

const ROUTE_RESTRICTION_HAIRCUT: f64 = 0.35;
const MIN_DIVERGENCE_BPS: f64 = 12.0;
const NETWORK_COST_USD: f64 = 0.04;

/// Runs quote-arb backtest over synthetic replay skew events.
pub fn run_quote_arb_backtest(cfg: Arc<SystemConfig>, dataset: &ReplayDataset) -> StrategyMetrics {
    let trade_size = cfg.execution.simulation_initial_amount_usd;
    let min_profit = cfg.execution.min_profit_threshold_usd;

    let mut winning = 0u64;
    let mut losing = 0u64;
    let mut rejected = 0u64;
    let mut total = 0u64;
    let mut net_pnl = 0.0_f64;
    let mut gross_pnl = 0.0_f64;
    let mut fees = 0.0_f64;
    let mut equity = vec![0.0_f64];
    let mut last_pair_skew: [f64; 5] = [1.0; 5];

    for (_ts, event) in &dataset.events {
        let ReplayEvent::Market(me) = event else {
            continue;
        };
        let MarketEvent::PoolUpdate(update) = me else {
            continue;
        };
        let Some(pool) = update.pool else {
            continue;
        };
        let pair = pool.as_bytes()[0] as usize;
        if pair >= 5 {
            continue;
        }

        let sqrt_price = update.sqrt_price.unwrap_or(1) as f64;
        if sqrt_price <= 0.0 {
            continue;
        }

        let mid = sqrt_price;
        let prev = last_pair_skew[pair];
        last_pair_skew[pair] = mid;

        let divergence_bps = ((mid / prev - 1.0).abs() * 10_000.0).min(500.0);
        if divergence_bps < MIN_DIVERGENCE_BPS {
            continue;
        }

        total += 1;
        let unrestricted_edge = trade_size * (divergence_bps / 10_000.0);
        let restricted_edge = unrestricted_edge * (1.0 - ROUTE_RESTRICTION_HAIRCUT);
        let fee = trade_size * 0.001 + NETWORK_COST_USD;
        fees += fee;
        gross_pnl += unrestricted_edge;

        let net = restricted_edge - fee;
        net_pnl += net;
        equity.push(net_pnl);

        if net >= min_profit {
            winning += 1;
        } else if net > 0.0 {
            rejected += 1;
        } else {
            losing += 1;
        }
    }

    let mut metrics = StrategyMetrics {
        strategy: "quote_arb".to_owned(),
        total_trades: total,
        winning_trades: winning,
        losing_trades: losing,
        rejected_trades: rejected,
        gross_pnl_usd: gross_pnl,
        net_pnl_usd: net_pnl,
        total_fees_usd: fees,
        win_rate: 0.0,
        avg_net_per_trade_usd: 0.0,
        trades_per_hour: 0.0,
        trades_per_day: 0.0,
        max_drawdown_usd: 0.0,
        sharpe_approx: 0.0,
    };
    metrics.finalize(dataset.duration_hours(), &equity);
    metrics
}
