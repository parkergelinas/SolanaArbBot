//! Pure feature-extraction functions — shared by scanner and backtest binaries.
#![allow(dead_code)]

use crate::types::{BookLevel, Trade};

/// Depth (total size) of levels within `cents` ticks of the best price.
pub fn depth_within(levels: &[BookLevel], cents: f64) -> f64 {
    let Some(first) = levels.first() else { return 0.0 };
    let best: f64 = first.price.parse().unwrap_or(0.0);
    let threshold = cents / 100.0;
    levels
        .iter()
        .filter(|l| {
            let p: f64 = l.price.parse().unwrap_or(0.0);
            (p - best).abs() <= threshold
        })
        .map(|l| l.size.parse::<f64>().unwrap_or(0.0))
        .sum()
}

/// Total size across all levels.
pub fn total_depth(levels: &[BookLevel]) -> f64 {
    levels
        .iter()
        .map(|l| l.size.parse::<f64>().unwrap_or(0.0))
        .sum()
}

/// Serialize up to `n` levels as [[price, size]] pairs.
pub fn top_levels(levels: &[BookLevel], n: usize) -> Vec<[f64; 2]> {
    levels
        .iter()
        .take(n)
        .filter_map(|l| {
            let p = l.price.parse::<f64>().ok()?;
            let s = l.size.parse::<f64>().ok()?;
            Some([p, s])
        })
        .collect()
}

/// Volume-weighted midpoint from the top `n` levels on each side.
/// Gives a better estimate of fair value than simple (bid+ask)/2 when
/// the book is uneven.
pub fn weighted_mid(bids: &[BookLevel], asks: &[BookLevel], n: usize) -> Option<f64> {
    let bid_levels = top_levels(bids, n);
    let ask_levels = top_levels(asks, n);
    if bid_levels.is_empty() || ask_levels.is_empty() {
        return None;
    }
    let bid_vwap = vwap(&bid_levels);
    let ask_vwap = vwap(&ask_levels);
    let bid_vol: f64 = bid_levels.iter().map(|l| l[1]).sum();
    let ask_vol: f64 = ask_levels.iter().map(|l| l[1]).sum();
    let total = bid_vol + ask_vol;
    if total == 0.0 {
        return None;
    }
    Some((bid_vwap * bid_vol + ask_vwap * ask_vol) / total)
}

fn vwap(levels: &[[f64; 2]]) -> f64 {
    let vol: f64 = levels.iter().map(|l| l[1]).sum();
    if vol == 0.0 {
        return 0.0;
    }
    levels.iter().map(|l| l[0] * l[1]).sum::<f64>() / vol
}

/// Bid depth / (bid depth + ask depth) within `cents` of each side's best.
/// 0.0 = all size is on ask side, 1.0 = all size is on bid side.
pub fn book_imbalance(bids: &[BookLevel], asks: &[BookLevel], cents: f64) -> Option<f64> {
    let bid_d = depth_within(bids, cents);
    let ask_d = depth_within(asks, cents);
    let total = bid_d + ask_d;
    if total == 0.0 {
        None
    } else {
        Some(bid_d / total)
    }
}

pub struct TradeStats {
    pub count: usize,
    pub buy_vol: f64,
    pub sell_vol: f64,
    pub buy_ratio: Option<f64>,
    pub avg_price: Option<f64>,
    pub vwap: Option<f64>,
    pub last_ts: Option<i64>,
}

/// Aggregate recent trades for a specific `token_id`.
pub fn trade_stats(trades: &[Trade], token_id: &str) -> TradeStats {
    let token_trades: Vec<&Trade> = trades
        .iter()
        .filter(|t| t.asset.as_deref() == Some(token_id))
        .collect();

    if token_trades.is_empty() {
        return TradeStats {
            count: 0,
            buy_vol: 0.0,
            sell_vol: 0.0,
            buy_ratio: None,
            avg_price: None,
            vwap: None,
            last_ts: None,
        };
    }

    let mut buy_vol = 0.0f64;
    let mut sell_vol = 0.0f64;
    let mut price_vol = 0.0f64;
    let mut total_vol = 0.0f64;
    let mut price_sum = 0.0f64;
    let mut last_ts: Option<i64> = None;

    for t in &token_trades {
        let size = t.size.unwrap_or(0.0);
        let price = t.price.unwrap_or(0.0);
        match t.side.as_deref() {
            Some("BUY") => buy_vol += size,
            Some("SELL") => sell_vol += size,
            _ => {}
        }
        price_vol += price * size;
        total_vol += size;
        price_sum += price;

        if let Some(ts) = t.timestamp {
            last_ts = Some(last_ts.map_or(ts, |prev: i64| prev.max(ts)));
        }
    }

    let total_flow = buy_vol + sell_vol;
    TradeStats {
        count: token_trades.len(),
        buy_vol,
        sell_vol,
        buy_ratio: if total_flow > 0.0 {
            Some(buy_vol / total_flow)
        } else {
            None
        },
        avg_price: if !token_trades.is_empty() {
            Some(price_sum / token_trades.len() as f64)
        } else {
            None
        },
        vwap: if total_vol > 0.0 {
            Some(price_vol / total_vol)
        } else {
            None
        },
        last_ts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BookLevel, Trade};

    fn lvl(price: &str, size: &str) -> BookLevel {
        BookLevel { price: price.into(), size: size.into() }
    }

    fn trade(side: &str, size: f64, price: f64, asset: &str, ts: i64) -> Trade {
        Trade {
            side: Some(side.into()),
            size: Some(size),
            price: Some(price),
            timestamp: Some(ts),
            asset: Some(asset.into()),
            outcome: None,
        }
    }

    // ── depth_within ──────────────────────────────────────────────────────────

    #[test]
    fn depth_within_empty() {
        assert_eq!(depth_within(&[], 5.0), 0.0);
    }

    #[test]
    fn depth_within_single_best() {
        let levels = vec![lvl("0.50", "100.0")];
        // Only one level at best; within any threshold it's included.
        assert_eq!(depth_within(&levels, 1.0), 100.0);
    }

    #[test]
    fn depth_within_threshold_boundary() {
        // best = 0.50; threshold = 1¢ = 0.01
        // 0.505 → within (distance 0.005 < 0.01) ✓
        // 0.489 → outside (distance 0.011 > 0.01) ✗
        // Avoids testing the exact floating-point boundary 0.50-0.49,
        // which is not exactly 0.01 in IEEE 754.
        let levels = vec![lvl("0.50", "10.0"), lvl("0.505", "20.0"), lvl("0.489", "30.0")];
        let d = depth_within(&levels, 1.0);
        assert!((d - 30.0).abs() < 1e-9, "expected 30, got {d}");
    }

    #[test]
    fn depth_within_5c() {
        // best = 0.60; 5¢ = 0.05 → levels from 0.55..=0.60 included
        let levels = vec![
            lvl("0.60", "5.0"),
            lvl("0.56", "10.0"),
            lvl("0.55", "15.0"), // exactly at boundary
            lvl("0.54", "99.0"), // outside
        ];
        let d = depth_within(&levels, 5.0);
        assert!((d - 30.0).abs() < 1e-9, "expected 30, got {d}");
    }

    // ── total_depth ───────────────────────────────────────────────────────────

    #[test]
    fn total_depth_empty() {
        assert_eq!(total_depth(&[]), 0.0);
    }

    #[test]
    fn total_depth_sums_all() {
        let levels = vec![lvl("0.50", "10.0"), lvl("0.48", "20.0"), lvl("0.45", "30.5")];
        let d = total_depth(&levels);
        assert!((d - 60.5).abs() < 1e-9);
    }

    #[test]
    fn total_depth_ignores_unparseable_size() {
        let levels = vec![lvl("0.50", "bad"), lvl("0.48", "20.0")];
        assert!((total_depth(&levels) - 20.0).abs() < 1e-9);
    }

    // ── top_levels ────────────────────────────────────────────────────────────

    #[test]
    fn top_levels_truncates_to_n() {
        let levels: Vec<BookLevel> = (0..20).map(|i| lvl(&format!("0.{i:02}"), "1.0")).collect();
        let top = top_levels(&levels, 10);
        assert_eq!(top.len(), 10);
    }

    #[test]
    fn top_levels_fewer_than_n() {
        let levels = vec![lvl("0.50", "5.0"), lvl("0.49", "3.0")];
        let top = top_levels(&levels, 10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0], [0.50, 5.0]);
    }

    #[test]
    fn top_levels_skips_unparseable() {
        let levels = vec![lvl("bad", "5.0"), lvl("0.49", "3.0"), lvl("0.48", "bad")];
        let top = top_levels(&levels, 10);
        // Only the fully parseable level passes.
        assert_eq!(top.len(), 1);
        assert_eq!(top[0], [0.49, 3.0]);
    }

    // ── weighted_mid ─────────────────────────────────────────────────────────

    #[test]
    fn weighted_mid_empty_bids() {
        let asks = vec![lvl("0.52", "100.0")];
        assert!(weighted_mid(&[], &asks, 5).is_none());
    }

    #[test]
    fn weighted_mid_empty_asks() {
        let bids = vec![lvl("0.48", "100.0")];
        assert!(weighted_mid(&bids, &[], 5).is_none());
    }

    #[test]
    fn weighted_mid_equal_volume() {
        // bids: 0.48 @ 50 | asks: 0.52 @ 50 → wmid = (0.48+0.52)/2 = 0.50
        let bids = vec![lvl("0.48", "50.0")];
        let asks = vec![lvl("0.52", "50.0")];
        let wmid = weighted_mid(&bids, &asks, 5).unwrap();
        assert!((wmid - 0.50).abs() < 1e-9, "expected 0.50, got {wmid}");
    }

    #[test]
    fn weighted_mid_skewed_to_bids() {
        // bids: 0.40 @ 90 | asks: 0.60 @ 10 → bids dominate → wmid close to 0.40
        let bids = vec![lvl("0.40", "90.0")];
        let asks = vec![lvl("0.60", "10.0")];
        let wmid = weighted_mid(&bids, &asks, 5).unwrap();
        // wmid = (0.40*90 + 0.60*10) / 100 = (36+6)/100 = 0.42
        assert!((wmid - 0.42).abs() < 1e-9, "expected 0.42, got {wmid}");
    }

    // ── book_imbalance ────────────────────────────────────────────────────────

    #[test]
    fn book_imbalance_both_empty() {
        assert!(book_imbalance(&[], &[], 10.0).is_none());
    }

    #[test]
    fn book_imbalance_all_bids() {
        let bids = vec![lvl("0.50", "100.0")];
        let imb = book_imbalance(&bids, &[], 10.0);
        // ask side empty → depth_within returns 0 → total = 100
        assert!(imb.is_some());
        assert!((imb.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn book_imbalance_balanced() {
        let bids = vec![lvl("0.50", "50.0")];
        let asks = vec![lvl("0.51", "50.0")];
        let imb = book_imbalance(&bids, &asks, 5.0).unwrap();
        assert!((imb - 0.5).abs() < 1e-9, "expected 0.5, got {imb}");
    }

    // ── trade_stats ───────────────────────────────────────────────────────────

    #[test]
    fn trade_stats_empty() {
        let stats = trade_stats(&[], "tok1");
        assert_eq!(stats.count, 0);
        assert_eq!(stats.buy_vol, 0.0);
        assert!(stats.buy_ratio.is_none());
        assert!(stats.vwap.is_none());
    }

    #[test]
    fn trade_stats_filters_by_token() {
        let trades = vec![
            trade("BUY", 10.0, 0.60, "tok1", 1000),
            trade("SELL", 20.0, 0.70, "tok2", 1001), // different token — excluded
        ];
        let stats = trade_stats(&trades, "tok1");
        assert_eq!(stats.count, 1);
        assert_eq!(stats.buy_vol, 10.0);
        assert_eq!(stats.sell_vol, 0.0);
    }

    #[test]
    fn trade_stats_buy_ratio() {
        let trades = vec![
            trade("BUY", 30.0, 0.60, "tok1", 1000),
            trade("SELL", 70.0, 0.50, "tok1", 1001),
        ];
        let stats = trade_stats(&trades, "tok1");
        let ratio = stats.buy_ratio.unwrap();
        assert!((ratio - 0.30).abs() < 1e-9, "expected 0.30, got {ratio}");
    }

    #[test]
    fn trade_stats_vwap() {
        // VWAP = (0.60*10 + 0.40*40) / 50 = (6+16)/50 = 0.44
        let trades = vec![
            trade("BUY", 10.0, 0.60, "tok1", 1000),
            trade("SELL", 40.0, 0.40, "tok1", 1001),
        ];
        let stats = trade_stats(&trades, "tok1");
        let vwap = stats.vwap.unwrap();
        assert!((vwap - 0.44).abs() < 1e-9, "expected 0.44, got {vwap}");
    }

    #[test]
    fn trade_stats_last_ts_is_max() {
        let trades = vec![
            trade("BUY", 10.0, 0.60, "tok1", 500),
            trade("BUY", 10.0, 0.61, "tok1", 1500),
            trade("BUY", 10.0, 0.62, "tok1", 1000),
        ];
        let stats = trade_stats(&trades, "tok1");
        assert_eq!(stats.last_ts, Some(1500));
    }
}
