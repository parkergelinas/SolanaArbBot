//! PnL attribution summary produced by `ScalpEngine::pnl_summary`.

/// Aggregated PnL and trade statistics for a `ScalpEngine` session.
///
/// All `*_bps` fields are summed across executed (non-rejected) trades.
/// Rejected trades are counted but not included in PnL totals.
#[derive(Clone, Debug, Default)]
pub struct PnlSummary {
    /// Total trades evaluated (executed + rejected).
    pub total_trades: u64,
    /// Executed trades where `pnl_bps > 0`.
    pub winning_trades: u64,
    /// Executed trades where `pnl_bps <= 0`.
    pub losing_trades: u64,
    /// Trades rejected by the filter chain.
    pub rejected_trades: u64,
    /// Sum of `pnl_bps` across all executed trades (before fee deduction).
    pub gross_pnl_bps: f64,
    /// Sum of `fee_paid_bps` across all executed trades.
    pub total_fees_bps: f64,
    /// `gross_pnl_bps − total_fees_bps`.
    pub net_pnl_bps: f64,
    /// `winning_trades / executed_trades`; 0.0 when no trades have executed.
    pub win_rate: f64,
    /// Mean `net_edge_bps` captured across executed trades.
    pub avg_edge_captured_bps: f64,
    /// `rejected_trades / total_trades`; 0.0 when no trades have been seen.
    pub rejection_rate: f64,
}
