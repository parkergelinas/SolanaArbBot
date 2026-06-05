//! Scalping strategy backtest runner.

use std::sync::Arc;

use config::SystemConfig;
use scalper::ScalpEngine;
use signals::SignalEngine;

use crate::dataset::{to_signal_input, ReplayDataset};
use crate::metrics::StrategyMetrics;

/// Pool TVL lookup keyed by pool pubkey bytes.
fn pool_tvl(pair_byte: u8) -> f64 {
    800_000.0 + f64::from(pair_byte) * 100_000.0
}

/// Runs the scalping backtest over `dataset`.
pub fn run_scalp_backtest(cfg: Arc<SystemConfig>, dataset: &ReplayDataset) -> StrategyMetrics {
    let (engine, _rx) = SignalEngine::new(cfg.signal_engine.clone());
    let scalp = ScalpEngine::new(Arc::clone(&cfg));

    let mut equity = vec![0.0_f64];
    let mut cumulative = 0.0_f64;
    let mut winning = 0u64;
    let mut losing = 0u64;
    let mut rejected = 0u64;
    let mut total = 0u64;
    let mut total_fees = 0.0_f64;
    let mut gross = 0.0_f64;

    for (ts, event) in &dataset.events {
        let Some(input) = to_signal_input(event) else {
            continue;
        };

        let signals = engine.process_at(input, *ts);

        for signal in signals {
            let pool_byte = signal.pool_address.as_bytes()[0];
            let features = engine
                .feature_store()
                .compute(signal.pool_address, *ts, &cfg.signal_engine);

            total += 1;
            if let Some(result) = scalp.evaluate(signal, features, pool_tvl(pool_byte), *ts) {
                if result.rejected {
                    rejected += 1;
                } else {
                    let pnl_usd =
                        result.candidate.trade_size_usd * (result.pnl_bps / 10_000.0);
                    let fee_usd = result.candidate.trade_size_usd
                        * (result.fee_paid_bps * 2.0 / 10_000.0);
                    gross += pnl_usd;
                    total_fees += fee_usd;
                    let net = pnl_usd - fee_usd;
                    cumulative += net;
                    if net > 0.0 {
                        winning += 1;
                    } else {
                        losing += 1;
                    }
                }
            } else {
                rejected += 1;
            }
        }

        equity.push(cumulative);
    }

    let summary = scalp.pnl_summary();
    if rejected < summary.rejected_trades {
        rejected = summary.rejected_trades;
    }

    let mut metrics = StrategyMetrics {
        strategy: "scalping".to_owned(),
        total_trades: total,
        winning_trades: winning,
        losing_trades: losing,
        rejected_trades: rejected,
        gross_pnl_usd: gross,
        net_pnl_usd: cumulative,
        total_fees_usd: total_fees,
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
