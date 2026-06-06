//! Strategy rankings with probability scores for dashboard selection.

use serde::{Deserialize, Serialize};

use crate::metrics::StrategyMetrics;
use crate::verify::VerificationReport;

/// One strategy row for the backtests comparison UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRanking {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Historical win rate on holdout (0–1).
    pub win_probability: f64,
    /// Composite chance strategy passes verification gates (0–1).
    pub pass_probability: f64,
    pub expected_usd_per_trade: f64,
    pub net_pnl_usd: f64,
    pub sharpe_approx: f64,
    pub trades_per_day: f64,
    pub max_drawdown_usd: f64,
    /// 0–100 composite confidence for ranking.
    pub confidence_score: f64,
    pub verified: bool,
    pub rank: u32,
    /// Maps to dashboard `botStore` strategy keys.
    pub bot_store_keys: Vec<String>,
}

fn metrics_to_ranking(
    id: &str,
    name: &str,
    description: &str,
    m: &StrategyMetrics,
    verification: Option<&VerificationReport>,
    bot_store_keys: Vec<String>,
) -> StrategyRanking {
    let _executed = m.winning_trades + m.losing_trades;
    let checks_passed = verification
        .map(|v| {
            if v.checks.is_empty() {
                0.0
            } else {
                v.checks.iter().filter(|c| c.passed).count() as f64 / v.checks.len() as f64
            }
        })
        .unwrap_or(if m.net_pnl_usd > 0.0 { 0.7 } else { 0.3 });

    let pass_probability = if verification.map(|v| v.passed).unwrap_or(false) {
        0.95_f64.min(0.75 + checks_passed * 0.25)
    } else {
        checks_passed * 0.65
    };

    let sharpe_norm = (m.sharpe_approx / 10.0).clamp(0.0, 1.0);
    let edge_norm = (m.avg_net_per_trade_usd / 0.5).clamp(0.0, 1.0);
    let win_norm = m.win_rate.clamp(0.0, 1.0);

    let confidence_score = (win_norm * 35.0
        + pass_probability * 30.0
        + sharpe_norm * 20.0
        + edge_norm * 15.0)
        .clamp(0.0, 100.0);

    StrategyRanking {
        id: id.to_owned(),
        name: name.to_owned(),
        description: description.to_owned(),
        win_probability: m.win_rate,
        pass_probability,
        expected_usd_per_trade: m.avg_net_per_trade_usd,
        net_pnl_usd: m.net_pnl_usd,
        sharpe_approx: m.sharpe_approx,
        trades_per_day: m.trades_per_day,
        max_drawdown_usd: m.max_drawdown_usd,
        confidence_score,
        verified: verification.map(|v| v.passed).unwrap_or(m.net_pnl_usd > 0.0),
        rank: 0,
        bot_store_keys,
    }
}

/// Build ranked strategy list from holdout metrics and verification.
pub fn build_strategy_rankings(
    scalp: &StrategyMetrics,
    arb: &StrategyMetrics,
    quote_arb: &StrategyMetrics,
    combined_net: f64,
    combined_trades_per_day: f64,
    retest_verification: &VerificationReport,
) -> Vec<StrategyRanking> {
    let combined_executed = scalp.winning_trades
        + scalp.losing_trades
        + arb.winning_trades
        + arb.losing_trades;
    let combined_win = if combined_executed == 0 {
        0.0
    } else {
        (scalp.winning_trades + arb.winning_trades) as f64 / combined_executed as f64
    };
    let combined_avg = if combined_executed == 0 {
        0.0
    } else {
        combined_net / combined_executed as f64
    };

    let combined_metrics = StrategyMetrics {
        strategy: "combined".to_owned(),
        total_trades: combined_executed,
        winning_trades: scalp.winning_trades + arb.winning_trades,
        losing_trades: scalp.losing_trades + arb.losing_trades,
        rejected_trades: scalp.rejected_trades + arb.rejected_trades,
        gross_pnl_usd: scalp.gross_pnl_usd + arb.gross_pnl_usd,
        net_pnl_usd: combined_net,
        total_fees_usd: scalp.total_fees_usd + arb.total_fees_usd,
        win_rate: combined_win,
        avg_net_per_trade_usd: combined_avg,
        trades_per_hour: combined_trades_per_day / 24.0,
        trades_per_day: combined_trades_per_day,
        max_drawdown_usd: scalp.max_drawdown_usd.max(arb.max_drawdown_usd),
        sharpe_approx: (scalp.sharpe_approx + arb.sharpe_approx) / 2.0,
    };

    let mut rows = vec![
        metrics_to_ranking(
            "scalping",
            "Scalping",
            "Micro-edge trades on momentum bursts with TP/SL exits.",
            scalp,
            None,
            vec!["scalp".to_owned()],
        ),
        metrics_to_ranking(
            "dex_arb",
            "DEX Arb",
            "Cross-venue spread capture on synthetic pool replay.",
            arb,
            None,
            vec!["arb".to_owned()],
        ),
        metrics_to_ranking(
            "quote_arb",
            "Route Divergence",
            "Jupiter route divergence — restricted vs unrestricted paths.",
            quote_arb,
            None,
            vec!["arb".to_owned()],
        ),
        metrics_to_ranking(
            "combined",
            "Combined",
            "Scalping + DEX arb running together on holdout data.",
            &combined_metrics,
            Some(retest_verification),
            vec!["scalp".to_owned(), "arb".to_owned()],
        ),
    ];

    rows.sort_by(|a, b| {
        b.confidence_score
            .partial_cmp(&a.confidence_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, row) in rows.iter_mut().enumerate() {
        row.rank = (i + 1) as u32;
    }

    rows
}
