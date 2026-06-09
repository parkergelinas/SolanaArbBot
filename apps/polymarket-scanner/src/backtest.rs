mod client;
mod dataset;
mod features;
mod scanner;
mod types;

use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

use client::PolyClient;
use types::BacktestRow;

fn api_key() -> Option<String> {
    std::env::var("POLY_API_KEY").ok().filter(|s| !s.is_empty())
}

fn out_path() -> PathBuf {
    std::env::var("POLY_BACKTEST_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/polymarket_backtest.jsonl"))
}

fn pages() -> usize {
    std::env::var("POLY_BACKTEST_PAGES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10) // 10 pages × 100 markets = up to 1 000 resolved markets
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("polymarket_scanner=info,warn")),
        )
        .init();

    let path = out_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create data dir")?;
    }

    let client = PolyClient::new(api_key())?;

    info!(pages = pages(), "fetching resolved markets");
    let markets = client.fetch_resolved_markets(pages()).await?;
    info!("processing {} resolved markets", markets.len());

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;

    let mut written = 0usize;
    let mut skipped = 0usize;

    for (idx, market) in markets.iter().enumerate() {
        info!(
            idx,
            total = markets.len(),
            condition_id = %market.condition_id,
            question = %market.question.chars().take(60).collect::<String>(),
            "processing"
        );

        let token_ids = match &market.clob_token_ids {
            Some(ids) if ids.len() >= 2 => ids.clone(),
            _ => {
                warn!(condition_id = %market.condition_id, "missing token ids, skipping");
                skipped += 1;
                continue;
            }
        };

        let outcomes_meta = market
            .outcomes
            .clone()
            .unwrap_or_else(|| vec!["Yes".into(), "No".into()]);

        let outcome_prices = match &market.outcome_prices {
            Some(p) => p.clone(),
            None => {
                skipped += 1;
                continue;
            }
        };

        // Find the winner: the token whose final price == "1"
        let winner_index = match outcome_prices.iter().position(|p| p == "1") {
            Some(i) => i,
            None => {
                warn!(condition_id = %market.condition_id, "no clear winner in outcomePrices");
                skipped += 1;
                continue;
            }
        };

        let winner_token_id = &token_ids[winner_index];
        let loser_token_id = token_ids.get(1 - winner_index.min(1));
        let winner_outcome = outcomes_meta
            .get(winner_index)
            .cloned()
            .unwrap_or_else(|| "?".into());

        // Fetch all available trades for this market (500 cap)
        let trades = match client.fetch_trades(&market.condition_id, 500).await {
            Ok(t) => t,
            Err(e) => {
                warn!(condition_id = %market.condition_id, "trades failed: {e}");
                vec![]
            }
        };

        // Separate trades by token
        let winner_trades: Vec<&types::Trade> = trades
            .iter()
            .filter(|t| t.asset.as_deref() == Some(winner_token_id))
            .collect();
        let loser_trades: Vec<&types::Trade> = trades
            .iter()
            .filter(|t| {
                loser_token_id.map_or(false, |lt| t.asset.as_deref() == Some(lt))
            })
            .collect();

        let trade_count = trades.len();
        let trade_history_truncated = trade_count >= 500;

        // Time span of fetched trades
        let all_ts: Vec<i64> = trades.iter().filter_map(|t| t.timestamp).collect();
        let trade_span_hours = if all_ts.len() >= 2 {
            let span = all_ts.iter().max().unwrap() - all_ts.iter().min().unwrap();
            Some(span as f64 / 3600.0)
        } else {
            None
        };

        // Market duration
        let duration_hours = market
            .start_date
            .as_deref()
            .zip(market.closed_time.as_deref())
            .and_then(|(s, c)| {
                let start = chrono::DateTime::parse_from_rfc3339(s).ok()?;
                let end = chrono::DateTime::parse_from_rfc3339(c).ok()?;
                Some((end - start).num_seconds() as f64 / 3600.0)
            });

        // All-time stats for winner and loser
        let all_stats_winner = agg_trades(&winner_trades);
        let all_stats_loser = agg_trades(&loser_trades);

        let all_buy_ratio = {
            let total_buy: f64 = winner_trades
                .iter()
                .chain(loser_trades.iter())
                .filter(|t| t.side.as_deref() == Some("BUY"))
                .filter_map(|t| t.size)
                .sum();
            let total_vol: f64 = winner_trades
                .iter()
                .chain(loser_trades.iter())
                .filter_map(|t| t.size)
                .sum();
            if total_vol > 0.0 {
                Some(total_buy / total_vol)
            } else {
                None
            }
        };

        // Window stats: split fetched winner trades into early/late thirds by timestamp
        let (early_winner, late_winner) = time_windows(&winner_trades);
        let early_stats = agg_trades(&early_winner);
        let late_stats = agg_trades(&late_winner);

        let price_start_winner = winner_trades
            .iter()
            .min_by_key(|t| t.timestamp.unwrap_or(i64::MAX))
            .and_then(|t| t.price);
        let price_end_winner = winner_trades
            .iter()
            .max_by_key(|t| t.timestamp.unwrap_or(i64::MIN))
            .and_then(|t| t.price);
        let price_drift = match (price_start_winner, price_end_winner) {
            (Some(s), Some(e)) => Some(e - s),
            _ => None,
        };
        let momentum_winner = match (early_stats.vwap, late_stats.vwap) {
            (Some(e), Some(l)) => Some(l - e),
            _ => None,
        };

        // Rate-limit: don't hammer the API
        tokio::time::sleep(Duration::from_millis(200)).await;

        let row = BacktestRow {
            condition_id: market.condition_id.clone(),
            question: market.question.clone(),
            slug: market.slug.clone(),
            start_date: market.start_date.clone(),
            closed_time: market.closed_time.clone(),
            winner_index,
            winner_outcome,
            winner_final_price: 1.0,
            duration_hours,
            volume_total: market.volume_num,
            competitive: market.competitive,
            maker_fee_bps: market.maker_base_fee,
            taker_fee_bps: market.taker_base_fee,
            neg_risk: market.neg_risk.unwrap_or(false),
            trade_count,
            trade_history_truncated,
            trade_span_hours,
            all_buy_ratio,
            all_vwap_winner: all_stats_winner.vwap,
            all_vwap_loser: all_stats_loser.vwap,
            all_vol_winner: all_stats_winner.total_vol,
            all_vol_loser: all_stats_loser.total_vol,
            early_buy_ratio_winner: early_stats.buy_ratio,
            early_vwap_winner: early_stats.vwap,
            early_price_winner: price_start_winner,
            late_buy_ratio_winner: late_stats.buy_ratio,
            late_vwap_winner: late_stats.vwap,
            late_price_winner: price_end_winner,
            momentum_winner,
            price_start_winner,
            price_end_winner,
            price_drift,
            spread: market.spread,
            best_bid: market.best_bid,
            best_ask: market.best_ask,
            last_trade_price: market.last_trade_price,
            one_week_price_change: market.one_week_price_change,
        };

        use std::io::Write;
        let line = serde_json::to_string(&row).context("serialize row")?;
        writeln!(file, "{line}").context("write row")?;
        written += 1;

        if written % 50 == 0 {
            info!(written, skipped, "progress");
        }
    }

    info!(written, skipped, path = %path.display(), "backtest dataset complete");
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

struct Agg {
    vwap: Option<f64>,
    buy_ratio: Option<f64>,
    total_vol: f64,
}

fn agg_trades(trades: &[&types::Trade]) -> Agg {
    if trades.is_empty() {
        return Agg { vwap: None, buy_ratio: None, total_vol: 0.0 };
    }
    let mut price_vol = 0.0f64;
    let mut total_vol = 0.0f64;
    let mut buy_vol = 0.0f64;

    for t in trades {
        let size = t.size.unwrap_or(0.0);
        let price = t.price.unwrap_or(0.0);
        price_vol += price * size;
        total_vol += size;
        if t.side.as_deref() == Some("BUY") {
            buy_vol += size;
        }
    }

    Agg {
        vwap: if total_vol > 0.0 { Some(price_vol / total_vol) } else { None },
        buy_ratio: if total_vol > 0.0 { Some(buy_vol / total_vol) } else { None },
        total_vol,
    }
}

/// Split trades into first-third and last-third by timestamp.
fn time_windows<'a>(
    trades: &[&'a types::Trade],
) -> (Vec<&'a types::Trade>, Vec<&'a types::Trade>) {
    if trades.is_empty() {
        return (vec![], vec![]);
    }
    let mut sorted = trades.to_vec();
    sorted.sort_by_key(|t| t.timestamp.unwrap_or(0));

    let n = sorted.len();
    let third = (n / 3).max(1);
    let early = sorted[..third].to_vec();
    let late = sorted[n.saturating_sub(third)..].to_vec();
    (early, late)
}
