//! Trade candidate and fill result types.
//!
//! `TradeCandidate` is the fully-costed snapshot of a potential trade built
//! from a `SignalEvent` before any filter evaluation.  `TradeResult` records
//! what happened after paper simulation.

use common::Pubkey;
use signals::SignalEvent;

// ─────────────────────────────────────────────────────────────────────────────
// TradeCandidate
// ─────────────────────────────────────────────────────────────────────────────

/// A prospective trade fully annotated with cost estimates, built from a
/// `SignalEvent` before the filter chain runs.
///
/// All monetary values are in USD; all cost values are in basis points.
#[derive(Clone, Debug)]
pub struct TradeCandidate {
    /// The signal that triggered this candidate.
    pub signal: SignalEvent,
    /// Pool this trade targets.
    pub pool_address: Pubkey,
    /// Pool TVL in USD at evaluation time (used by `LiquidityFilter`).
    pub pool_tvl_usd: f64,
    /// Last observed price from the feature store (denominated in pool units).
    pub estimated_entry_price: f64,
    /// Position size in USD computed by the sizing model.
    pub trade_size_usd: f64,
    /// One-way Raydium price-impact estimate in basis points.
    pub estimated_slippage_bps: f64,
    /// One-way Jupiter fee estimate in basis points.
    pub estimated_fee_bps: f64,
    /// Full round-trip cost in basis points (2× slippage + 2× fees).
    pub estimated_round_trip_cost_bps: f64,
    /// `signal_strength_bps − estimated_round_trip_cost_bps`.
    pub net_edge_bps: f64,
    /// Microsecond timestamp when this candidate was constructed.
    pub created_at_micros: u64,
    /// Microsecond timestamp after which the signal is considered stale.
    pub expires_at_micros: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// TradeResult
// ─────────────────────────────────────────────────────────────────────────────

/// Outcome of a paper-simulated fill, including PnL attribution.
///
/// When `rejected` is `true`, price/cost/pnl fields are zero and
/// `reject_reason` contains the filter name and reason.
#[derive(Clone, Debug)]
pub struct TradeResult {
    /// The candidate that was evaluated.
    pub candidate: TradeCandidate,
    /// Simulated fill price after slippage (zero when rejected).
    pub fill_price: f64,
    /// Actual one-way slippage applied by the simulator in basis points.
    pub actual_slippage_bps: f64,
    /// Fee fraction applied by the simulator in basis points.
    pub fee_paid_bps: f64,
    /// `signal_strength_bps − (actual_slippage_bps·2 + fee_paid_bps·2 + mev_haircut_bps)`.
    pub pnl_bps: f64,
    /// `true` when the trade was rejected before simulation.
    pub rejected: bool,
    /// Human-readable rejection explanation (format: `"[FilterName] reason"`).
    pub reject_reason: Option<String>,
    /// Microsecond timestamp of the simulated execution.
    pub executed_at_micros: u64,
}
