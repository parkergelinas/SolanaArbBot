//! Append-only JSONL dataset writer — used by the scanner binary.
#![allow(dead_code)]

use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Context, Result};

use crate::types::MarketSnapshot;

pub struct DatasetWriter {
    writer: BufWriter<File>,
    rows_written: u64,
}

impl DatasetWriter {
    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("open dataset file: {}", path.display()))?;
        Ok(Self {
            writer: BufWriter::new(file),
            rows_written: 0,
        })
    }

    pub fn write_snapshot(&mut self, snap: &MarketSnapshot) -> Result<()> {
        let line = serde_json::to_string(snap).context("serialize snapshot")?;
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.rows_written += 1;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().context("flush dataset")
    }

    pub fn rows_written(&self) -> u64 {
        self.rows_written
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::io::{BufRead, BufReader};
    use tempfile::NamedTempFile;

    use crate::types::{MarketSnapshot, OutcomeSnapshot};

    fn dummy_snapshot() -> MarketSnapshot {
        MarketSnapshot {
            ts: Utc::now(),
            condition_id: "0xdeadbeef".into(),
            question: "Will X happen?".into(),
            slug: Some("will-x-happen".into()),
            event_title: None,
            end_date: Some("2099-01-01T00:00:00Z".into()),
            days_to_expiry: Some(365.0),
            neg_risk: false,
            maker_fee_bps: Some(100),
            taker_fee_bps: Some(200),
            competitive: Some(0.8),
            rewards_min_size: None,
            rewards_max_spread: None,
            volume_total: Some(100_000.0),
            volume_24h: Some(5_000.0),
            volume_1wk: None,
            volume_1mo: None,
            liquidity: Some(20_000.0),
            event_volume_24h: None,
            event_open_interest: None,
            event_comment_count: None,
            outcomes: vec![OutcomeSnapshot {
                token_id: "tok1".into(),
                outcome: "Yes".into(),
                price: Some(0.65),
                best_bid: Some(0.64),
                best_ask: Some(0.66),
                midpoint: Some(0.65),
                spread: Some(0.02),
                last_trade_price: Some(0.65),
                last_trade_side: Some("BUY".into()),
                bid_levels: vec![[0.64, 50.0]],
                ask_levels: vec![[0.66, 50.0]],
                total_bid_depth: 50.0,
                total_ask_depth: 50.0,
                depth_1c_bid: 50.0,
                depth_1c_ask: 50.0,
                depth_5c_bid: 50.0,
                depth_5c_ask: 50.0,
                depth_10c_bid: 50.0,
                depth_10c_ask: 50.0,
                book_imbalance: Some(0.5),
                weighted_mid: Some(0.65),
                price_change_1wk: None,
                price_change_1mo: None,
                price_change_1yr: None,
                trades_count: 10,
                trade_buy_vol: 300.0,
                trade_sell_vol: 200.0,
                trade_buy_ratio: Some(0.6),
                trade_avg_price: Some(0.65),
                trade_vwap: Some(0.65),
                last_trade_ts: Some(1_700_000_000),
                implied_prob: Some(0.65),
                prob_distance_from_50: Some(0.15),
                spread_pct: Some(0.031),
                fee_adjusted_edge: Some(0.02),
            }],
            price_sum: Some(1.0),
            price_sum_deviation: Some(0.0),
            spread_pct: None,
            vol_liq_ratio: Some(0.25),
        }
    }

    #[test]
    fn write_flush_and_count() {
        let tmp = NamedTempFile::new().expect("tempfile");
        let path = tmp.path();

        let mut writer = DatasetWriter::open(path).expect("open");
        assert_eq!(writer.rows_written(), 0);

        writer.write_snapshot(&dummy_snapshot()).expect("write 1");
        writer.write_snapshot(&dummy_snapshot()).expect("write 2");
        writer.flush().expect("flush");

        assert_eq!(writer.rows_written(), 2);

        // Verify the file contains valid JSON on two lines.
        let f = std::fs::File::open(path).expect("reopen");
        let lines: Vec<String> = BufReader::new(f)
            .lines()
            .map(|l| l.expect("read line"))
            .collect();
        assert_eq!(lines.len(), 2);

        for line in &lines {
            let v: serde_json::Value =
                serde_json::from_str(line).expect("line is valid JSON");
            assert_eq!(v["condition_id"], "0xdeadbeef");
        }
    }

    #[test]
    fn append_mode_accumulates_rows() {
        let tmp = NamedTempFile::new().expect("tempfile");
        let path = tmp.path().to_path_buf();

        {
            let mut w = DatasetWriter::open(&path).expect("open 1");
            w.write_snapshot(&dummy_snapshot()).expect("write");
            w.flush().expect("flush");
        }
        {
            let mut w = DatasetWriter::open(&path).expect("open 2");
            w.write_snapshot(&dummy_snapshot()).expect("write");
            w.flush().expect("flush");
        }

        let line_count = std::fs::read_to_string(&path)
            .expect("read")
            .lines()
            .count();
        assert_eq!(line_count, 2, "expected 2 appended lines");
    }
}
