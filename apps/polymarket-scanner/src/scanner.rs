//! Market scanner — used by the scanner binary.
#![allow(dead_code)]

use anyhow::Result;
use chrono::Utc;
use tracing::{debug, warn};

use crate::{
    client::PolyClient,
    features::{book_imbalance, depth_within, top_levels, total_depth, trade_stats, weighted_mid},
    types::{MarketSnapshot, OutcomeSnapshot},
};

/// Compute calendar days from now until `iso_date` (may be negative if past).
pub fn days_to_expiry(iso_date: &str) -> Option<f64> {
    let end = chrono::DateTime::parse_from_rfc3339(iso_date)
        .ok()
        .map(|d| d.with_timezone(&Utc))?;
    let now = Utc::now();
    let secs = (end - now).num_seconds() as f64;
    Some(secs / 86_400.0)
}

pub async fn scan_once(
    client: &PolyClient,
    min_volume_24h: f64,
) -> Result<Vec<MarketSnapshot>> {
    let markets = client.fetch_gamma_markets().await?;
    tracing::info!("fetched {} order-book markets from Gamma", markets.len());

    let ts = Utc::now();
    let mut snapshots = Vec::new();

    for market in &markets {
        // Volume gate: skip illiquid markets early.
        let vol_24h = market.volume24hr.unwrap_or(0.0);
        if vol_24h < min_volume_24h {
            debug!(condition_id = %market.condition_id, vol_24h, "skipping low-volume market");
            continue;
        }

        let token_ids = match &market.clob_token_ids {
            Some(ids) if !ids.is_empty() => ids.clone(),
            _ => {
                debug!(condition_id = %market.condition_id, "no clob token ids");
                continue;
            }
        };

        let outcomes_meta: Vec<String> = market
            .outcomes
            .clone()
            .unwrap_or_else(|| token_ids.iter().map(|_| "?".to_string()).collect());

        let outcome_prices: Vec<Option<f64>> = market
            .outcome_prices
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|s| s.parse::<f64>().ok())
            .collect();

        // Fetch recent trades once per market (shared across all outcome tokens).
        let trades = match client.fetch_trades(&market.condition_id, 100).await {
            Ok(t) => t,
            Err(e) => {
                warn!(condition_id = %market.condition_id, "trades fetch failed: {e}");
                vec![]
            }
        };

        let mut outcome_snapshots = Vec::new();
        let mut prices_for_sum: Vec<f64> = Vec::new();

        for (i, token_id) in token_ids.iter().enumerate() {
            let outcome_name = outcomes_meta
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("Outcome {i}"));
            let gamma_price = outcome_prices.get(i).copied().flatten();

            // Fetch book, midpoint, and last-trade concurrently for each token.
            let (book_res, mid_res, lt_res) = tokio::join!(
                client.fetch_book(token_id),
                client.fetch_midpoint(token_id),
                client.fetch_last_trade(token_id),
            );

            let book = match book_res {
                Ok(b) => b,
                Err(e) => {
                    warn!(token_id, "book fetch failed: {e}");
                    // Emit a partial outcome snapshot with what we have from Gamma.
                    let stats = trade_stats(&trades, token_id);
                    outcome_snapshots.push(OutcomeSnapshot {
                        token_id: token_id.clone(),
                        outcome: outcome_name,
                        price: gamma_price,
                        best_bid: None,
                        best_ask: None,
                        midpoint: None,
                        spread: None,
                        last_trade_price: None,
                        last_trade_side: None,
                        bid_levels: vec![],
                        ask_levels: vec![],
                        total_bid_depth: 0.0,
                        total_ask_depth: 0.0,
                        depth_1c_bid: 0.0,
                        depth_1c_ask: 0.0,
                        depth_5c_bid: 0.0,
                        depth_5c_ask: 0.0,
                        depth_10c_bid: 0.0,
                        depth_10c_ask: 0.0,
                        book_imbalance: None,
                        weighted_mid: None,
                        price_change_1wk: if i == 0 { market.one_week_price_change } else { None },
                        price_change_1mo: if i == 0 { market.one_month_price_change } else { None },
                        price_change_1yr: if i == 0 { market.one_year_price_change } else { None },
                        trades_count: stats.count,
                        trade_buy_vol: stats.buy_vol,
                        trade_sell_vol: stats.sell_vol,
                        trade_buy_ratio: stats.buy_ratio,
                        trade_avg_price: stats.avg_price,
                        trade_vwap: stats.vwap,
                        last_trade_ts: stats.last_ts,
                        implied_prob: gamma_price,
                        prob_distance_from_50: gamma_price.map(|p| (p - 0.5).abs()),
                        spread_pct: None,
                        fee_adjusted_edge: None,
                    });
                    continue;
                }
            };

            let midpoint: Option<f64> = mid_res
                .ok()
                .and_then(|m| m.mid)
                .and_then(|s| s.parse().ok());

            let (last_trade_price, last_trade_side) = lt_res
                .map(|lt| {
                    let p = lt.price.and_then(|s| s.parse::<f64>().ok());
                    (p, lt.side)
                })
                .unwrap_or((None, None));

            // Book features
            let best_bid = book.bids.first().and_then(|l| l.price.parse::<f64>().ok());
            let best_ask = book.asks.first().and_then(|l| l.price.parse::<f64>().ok());
            let spread = match (best_bid, best_ask) {
                (Some(b), Some(a)) => Some(a - b),
                _ => None,
            };
            let mid = midpoint.or_else(|| match (best_bid, best_ask) {
                (Some(b), Some(a)) => Some((b + a) / 2.0),
                _ => None,
            });

            let total_bid = total_depth(&book.bids);
            let total_ask = total_depth(&book.asks);
            let imbalance = book_imbalance(&book.bids, &book.asks, 10.0);
            let wmid = weighted_mid(&book.bids, &book.asks, 5);

            let spread_pct = match (spread, mid) {
                (Some(s), Some(m)) if m > 0.0 => Some(s / m),
                _ => None,
            };

            // Fee-adjusted edge: taker_base_fee is in basis points (100 bps = 1%).
            let fee_adjusted_edge = market.taker_base_fee.map(|f| f as f64 / 10_000.0);

            let stats = trade_stats(&trades, token_id);

            if let Some(m) = mid {
                prices_for_sum.push(m);
            }

            outcome_snapshots.push(OutcomeSnapshot {
                token_id: token_id.clone(),
                outcome: outcome_name,
                price: gamma_price,
                best_bid,
                best_ask,
                midpoint: mid,
                spread,
                last_trade_price,
                last_trade_side,
                bid_levels: top_levels(&book.bids, 10),
                ask_levels: top_levels(&book.asks, 10),
                total_bid_depth: total_bid,
                total_ask_depth: total_ask,
                depth_1c_bid: depth_within(&book.bids, 1.0),
                depth_1c_ask: depth_within(&book.asks, 1.0),
                depth_5c_bid: depth_within(&book.bids, 5.0),
                depth_5c_ask: depth_within(&book.asks, 5.0),
                depth_10c_bid: depth_within(&book.bids, 10.0),
                depth_10c_ask: depth_within(&book.asks, 10.0),
                book_imbalance: imbalance,
                weighted_mid: wmid,
                price_change_1wk: if i == 0 { market.one_week_price_change } else { None },
                price_change_1mo: if i == 0 { market.one_month_price_change } else { None },
                price_change_1yr: if i == 0 { market.one_year_price_change } else { None },
                trades_count: stats.count,
                trade_buy_vol: stats.buy_vol,
                trade_sell_vol: stats.sell_vol,
                trade_buy_ratio: stats.buy_ratio,
                trade_avg_price: stats.avg_price,
                trade_vwap: stats.vwap,
                last_trade_ts: stats.last_ts,
                implied_prob: mid,
                prob_distance_from_50: mid.map(|p| (p - 0.5).abs()),
                spread_pct,
                fee_adjusted_edge,
            });
        }

        // Market-level derived signals
        let price_sum = if prices_for_sum.len() >= 2 {
            Some(prices_for_sum.iter().sum())
        } else {
            None
        };
        let spread_pct = match (market.spread, market.best_bid, market.best_ask) {
            (Some(s), _, Some(a)) if a > 0.0 => Some(s / a),
            _ => None,
        };
        let vol_liq_ratio =
            match (market.volume24hr, market.liquidity_num.or(market.liquidity_clob)) {
                (Some(v), Some(l)) if l > 0.0 => Some(v / l),
                _ => None,
            };

        let event = market.events.as_deref().and_then(|e| e.first());
        let event_title = event.and_then(|e| e.title.clone());
        let event_vol_24h = event.and_then(|e| e.volume24hr);
        let event_oi = event.and_then(|e| e.open_interest);
        let event_comments = event.and_then(|e| e.comment_count);

        let end_date = market
            .end_date_iso
            .clone()
            .or_else(|| market.end_date.clone());
        let dte = end_date.as_deref().and_then(days_to_expiry);

        snapshots.push(MarketSnapshot {
            ts,
            condition_id: market.condition_id.clone(),
            question: market.question.clone(),
            slug: market.slug.clone(),
            event_title,
            end_date,
            days_to_expiry: dte,
            neg_risk: market.neg_risk.unwrap_or(false),
            maker_fee_bps: market.maker_base_fee,
            taker_fee_bps: market.taker_base_fee,
            competitive: market.competitive,
            rewards_min_size: market.rewards_min_size,
            rewards_max_spread: market.rewards_max_spread,
            volume_total: market.volume_num,
            volume_24h: market.volume24hr,
            volume_1wk: market.volume1wk,
            volume_1mo: market.volume1mo,
            liquidity: market.liquidity_num.or(market.liquidity_clob),
            event_volume_24h: event_vol_24h,
            event_open_interest: event_oi,
            event_comment_count: event_comments,
            outcomes: outcome_snapshots,
            price_sum,
            price_sum_deviation: price_sum.map(|s| s - 1.0),
            spread_pct,
            vol_liq_ratio,
        });
    }

    tracing::info!(
        markets_fetched = markets.len(),
        snapshots = snapshots.len(),
        "scan complete"
    );
    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use super::days_to_expiry;

    #[test]
    fn days_to_expiry_future() {
        // 2099-01-01 is far in the future — should give a large positive value
        let dte = days_to_expiry("2099-01-01T00:00:00Z").unwrap();
        assert!(dte > 1000.0, "expected many days, got {dte}");
    }

    #[test]
    fn days_to_expiry_past() {
        let dte = days_to_expiry("2000-01-01T00:00:00Z").unwrap();
        assert!(dte < 0.0, "expected negative days for past date, got {dte}");
    }

    #[test]
    fn days_to_expiry_invalid() {
        assert!(days_to_expiry("not-a-date").is_none());
        assert!(days_to_expiry("").is_none());
    }
}
