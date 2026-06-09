use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize a field that may be either a real JSON array or a
/// stringified JSON array (e.g. `"[\"Yes\",\"No\"]"`).
pub fn de_stringified_vec<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    use serde::de::Error;
    use serde_json::Value;

    let v = Option::<Value>::deserialize(deserializer)?;
    match v {
        None => Ok(None),
        Some(Value::Array(arr)) => {
            let items = arr
                .into_iter()
                .map(serde_json::from_value::<T>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(D::Error::custom)?;
            Ok(Some(items))
        }
        Some(Value::String(s)) => {
            let items = serde_json::from_str::<Vec<T>>(&s).map_err(D::Error::custom)?;
            Ok(Some(items))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected array or stringified array, got {other}"
        ))),
    }
}

// ── Gamma API ────────────────────────────────────────────────────────────────

// Rich deserialization structs: many fields are captured for completeness.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaMarket {
    pub condition_id: String,
    pub question: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub end_date_iso: Option<String>,
    pub closed_time: Option<String>,
    pub active: bool,
    pub closed: bool,
    pub archived: Option<bool>,
    pub accepting_orders: Option<bool>,
    pub enable_order_book: Option<bool>,
    pub neg_risk: Option<bool>,

    // Volume tiers
    pub volume_num: Option<f64>,
    pub volume24hr: Option<f64>,
    pub volume1wk: Option<f64>,
    pub volume1mo: Option<f64>,
    pub volume1yr: Option<f64>,

    // Liquidity
    pub liquidity_num: Option<f64>,
    pub liquidity_clob: Option<f64>,

    // Price signals from Gamma (pre-computed)
    pub spread: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub last_trade_price: Option<f64>,
    pub one_week_price_change: Option<f64>,
    pub one_month_price_change: Option<f64>,
    pub one_year_price_change: Option<f64>,

    // Market structure
    #[serde(default, deserialize_with = "de_stringified_vec")]
    pub outcome_prices: Option<Vec<String>>,
    #[serde(default, deserialize_with = "de_stringified_vec")]
    pub outcomes: Option<Vec<String>>,
    #[serde(default, deserialize_with = "de_stringified_vec")]
    pub clob_token_ids: Option<Vec<String>>,
    pub maker_base_fee: Option<i64>,
    pub taker_base_fee: Option<i64>,
    pub fees_enabled: Option<bool>,
    pub competitive: Option<f64>,
    pub rewards_min_size: Option<f64>,
    pub rewards_max_spread: Option<f64>,

    // Event context
    pub events: Option<Vec<GammaEvent>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaEvent {
    pub id: Option<String>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub volume: Option<f64>,
    pub volume24hr: Option<f64>,
    pub volume1wk: Option<f64>,
    pub liquidity: Option<f64>,
    pub open_interest: Option<f64>,
    pub comment_count: Option<u64>,
    pub competitive: Option<f64>,
    pub event_metadata: Option<EventMetadata>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMetadata {
    pub context_description: Option<String>,
}

// ── CLOB API ─────────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BookLevel {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Midpoint {
    pub mid: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LastTradePrice {
    pub price: Option<String>,
    pub side: Option<String>,
}

// ── Data API trades ───────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub side: Option<String>,
    pub size: Option<f64>,
    pub price: Option<f64>,
    pub timestamp: Option<i64>,
    pub asset: Option<String>,
    pub outcome: Option<String>,
}

// ── Canonical dataset row ─────────────────────────────────────────────────────

/// One row per scan per market written to the JSONL dataset.
#[derive(Debug, Clone, Serialize)]
pub struct MarketSnapshot {
    // ── Metadata ──────────────────────────────────────────────────────────────
    pub ts: DateTime<Utc>,
    pub condition_id: String,
    pub question: String,
    pub slug: Option<String>,
    pub event_title: Option<String>,
    pub end_date: Option<String>,
    pub days_to_expiry: Option<f64>,

    // ── Market structure ──────────────────────────────────────────────────────
    pub neg_risk: bool,
    pub maker_fee_bps: Option<i64>,
    pub taker_fee_bps: Option<i64>,
    pub competitive: Option<f64>,         // 0-1, how liquid/tight this market is
    pub rewards_min_size: Option<f64>,    // LP reward program params
    pub rewards_max_spread: Option<f64>,

    // ── Volume & liquidity ────────────────────────────────────────────────────
    pub volume_total: Option<f64>,
    pub volume_24h: Option<f64>,
    pub volume_1wk: Option<f64>,
    pub volume_1mo: Option<f64>,
    pub liquidity: Option<f64>,
    pub event_volume_24h: Option<f64>,    // parent event total
    pub event_open_interest: Option<f64>,
    pub event_comment_count: Option<u64>, // social signal

    // ── Per-outcome snapshots ─────────────────────────────────────────────────
    pub outcomes: Vec<OutcomeSnapshot>,

    // ── Market-level derived signals ──────────────────────────────────────────
    /// YES + NO prices (should be ≈ 1.0). >1.02 = potential arb.
    pub price_sum: Option<f64>,
    pub price_sum_deviation: Option<f64>, // price_sum - 1.0
    /// Spread as fraction of mid price (normalized spread).
    pub spread_pct: Option<f64>,
    /// Volume-to-liquidity ratio: how much this market trades relative to its depth.
    pub vol_liq_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutcomeSnapshot {
    pub token_id: String,
    pub outcome: String,

    // ── Raw prices ────────────────────────────────────────────────────────────
    pub price: Option<f64>,           // from market token list
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub midpoint: Option<f64>,        // from CLOB /midpoint
    pub spread: Option<f64>,          // best_ask - best_bid
    pub last_trade_price: Option<f64>,
    pub last_trade_side: Option<String>,

    // ── Order book depth ──────────────────────────────────────────────────────
    pub bid_levels: Vec<[f64; 2]>,    // [[price, size], ...] top 10
    pub ask_levels: Vec<[f64; 2]>,
    pub total_bid_depth: f64,
    pub total_ask_depth: f64,
    pub depth_1c_bid: f64,            // depth within 1¢ of best bid
    pub depth_1c_ask: f64,
    pub depth_5c_bid: f64,
    pub depth_5c_ask: f64,
    pub depth_10c_bid: f64,
    pub depth_10c_ask: f64,
    pub book_imbalance: Option<f64>,  // bid_depth / (bid+ask) at 10¢, 0=all asks, 1=all bids
    pub weighted_mid: Option<f64>,    // VWAP from top 5 levels each side

    // ── Price history / momentum ──────────────────────────────────────────────
    pub price_change_1wk: Option<f64>,
    pub price_change_1mo: Option<f64>,
    pub price_change_1yr: Option<f64>,

    // ── Recent trade flow (last 50 trades for this token) ─────────────────────
    pub trades_count: usize,
    pub trade_buy_vol: f64,
    pub trade_sell_vol: f64,
    pub trade_buy_ratio: Option<f64>, // buy_vol / (buy_vol + sell_vol)
    pub trade_avg_price: Option<f64>,
    pub trade_vwap: Option<f64>,      // volume-weighted avg price of recent trades
    pub last_trade_ts: Option<i64>,   // unix timestamp of most recent trade

    // ── Derived signals ───────────────────────────────────────────────────────
    pub implied_prob: Option<f64>,    // = midpoint (the market's best estimate)
    pub prob_distance_from_50: Option<f64>, // |prob - 0.5|, high = market has conviction
    pub spread_pct: Option<f64>,      // spread / midpoint, normalized cost to cross
    pub fee_adjusted_edge: Option<f64>, // what you need to beat just to break even
}

// ── Backtest labeled row ──────────────────────────────────────────────────────

/// One labeled row per resolved market written to the backtest JSONL.
/// Features are reconstructed from trade history; label is the known outcome.
#[derive(Debug, Serialize)]
pub struct BacktestRow {
    // ── Identity ──────────────────────────────────────────────────────────────
    pub condition_id: String,
    pub question: String,
    pub slug: Option<String>,
    pub start_date: Option<String>,
    pub closed_time: Option<String>,

    // ── Ground truth label ────────────────────────────────────────────────────
    /// Index of the winning outcome token (0 = first token, 1 = second).
    pub winner_index: usize,
    pub winner_outcome: String,
    /// Final settlement price of the winning token (always 1.0 for binary).
    pub winner_final_price: f64,

    // ── Market structure ──────────────────────────────────────────────────────
    pub duration_hours: Option<f64>,
    pub volume_total: Option<f64>,
    pub competitive: Option<f64>,
    pub maker_fee_bps: Option<i64>,
    pub taker_fee_bps: Option<i64>,
    pub neg_risk: bool,

    // ── Trade-reconstructed features ─────────────────────────────────────────
    /// Total trades fetched (capped at 500 by the API).
    pub trade_count: usize,
    /// True if the 500-trade cap was hit (history may be incomplete).
    pub trade_history_truncated: bool,
    /// Time span covered by fetched trades, in hours.
    pub trade_span_hours: Option<f64>,

    // All-time aggregates (across fetched trades)
    pub all_buy_ratio: Option<f64>,     // buy_vol / total_vol
    pub all_vwap_winner: Option<f64>,   // VWAP of winner token trades
    pub all_vwap_loser: Option<f64>,
    pub all_vol_winner: f64,
    pub all_vol_loser: f64,

    // Early-window (first 33% of fetched trades by time)
    pub early_buy_ratio_winner: Option<f64>,
    pub early_vwap_winner: Option<f64>,
    pub early_price_winner: Option<f64>, // first trade price

    // Late-window (last 33% of fetched trades by time)
    pub late_buy_ratio_winner: Option<f64>,
    pub late_vwap_winner: Option<f64>,
    pub late_price_winner: Option<f64>,  // last trade price before close

    // Momentum: late_vwap - early_vwap (positive = price rising toward 1)
    pub momentum_winner: Option<f64>,

    // Price at start vs end (for the winner token)
    pub price_start_winner: Option<f64>,
    pub price_end_winner: Option<f64>,
    pub price_drift: Option<f64>, // end - start

    // ── Gamma pre-computed signals ────────────────────────────────────────────
    pub spread: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub last_trade_price: Option<f64>,
    pub one_week_price_change: Option<f64>,
}
