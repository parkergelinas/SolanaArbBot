//! `ScalpEngine` — main coordinator for signal evaluation and paper trading.
//!
//! # Evaluation flow
//!
//! ```text
//! SignalEvent + ComputedFeatures + pool_tvl_usd
//!   │
//!   ├─ Signal expiry check (signal_max_age_secs)
//!   │
//!   ├─ compute_size (sizing model)
//!   │
//!   ├─ round_trip_cost_bps (microstructure)
//!   │
//!   ├─ TradeCandidate construction
//!   │
//!   ├─ TradeFilterChain::check_all
//!   │     ├─ REJECT → store rejected TradeResult, return None
//!   │     └─ PASS
//!   │           ↓
//!   ├─ PaperSimulator::simulate_fill
//!   │
//!   ├─ TradeFilterChain::record_trade
//!   │
//!   └─ store TradeResult → return Some(TradeResult)
//! ```
//!
//! No signing, no wallet, no RPC call ever happens here.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use config::SystemConfig;
use signals::{ComputedFeatures, SignalEvent};
use tracing::{debug, warn};

use crate::candidate::{TradeCandidate, TradeResult};
use crate::filters::TradeFilterChain;
use crate::microstructure::{jupiter_effective_fee, raydium_price_impact, round_trip_cost_bps};
use crate::pnl::PnlSummary;
use crate::simulator::PaperSimulator;
use crate::sizing;

/// Maximum number of trade results kept in the in-process history ring buffer.
const MAX_HISTORY: usize = 10_000;

// ─────────────────────────────────────────────────────────────────────────────
// ScalpEngine
// ─────────────────────────────────────────────────────────────────────────────

/// Top-level coordinator: evaluates signals, applies the filter chain, and
/// drives the paper simulator.
///
/// Thread-safe: holds only `Arc`/`Mutex` interior mutable state.  The filter
/// chain uses `DashMap` internally so no `Arc<RwLock<>>` is required on the
/// hot path.
pub struct ScalpEngine {
    filter_chain: TradeFilterChain,
    simulator: PaperSimulator,
    config: Arc<SystemConfig>,
    trade_history: Arc<Mutex<VecDeque<TradeResult>>>,
}

impl ScalpEngine {
    /// Create a new engine backed by `cfg`.
    pub fn new(cfg: Arc<SystemConfig>) -> Self {
        let filter_chain = TradeFilterChain::new(&cfg.scalper);
        Self {
            filter_chain,
            simulator: PaperSimulator,
            config: cfg,
            trade_history: Arc::new(Mutex::new(VecDeque::with_capacity(256))),
        }
    }

    /// Evaluate a signal and optionally produce a paper fill.
    ///
    /// Returns `None` when the signal is expired or any filter rejects the
    /// trade.  Returns `Some(TradeResult)` on a successful paper fill.
    ///
    /// All rejected trades are still recorded in the internal history for
    /// PnL attribution (see [`pnl_summary`]).
    pub fn evaluate(
        &self,
        signal: SignalEvent,
        features: ComputedFeatures,
        pool_tvl_usd: f64,
        now_micros: u64,
    ) -> Option<TradeResult> {
        let sc = &self.config.scalper;

        let expires_at =
            signal.timestamp_micros.saturating_add(sc.signal_max_age_secs.saturating_mul(1_000_000));

        if now_micros > expires_at {
            debug!(signal_id = signal.signal_id, "dropping expired signal");
            return None;
        }

        // ── Position sizing ───────────────────────────────────────────────
        let trade_size_usd =
            sizing::compute_size(&signal, self.config.portfolio.capital_usd, sc);

        // ── Cost estimation ───────────────────────────────────────────────
        let one_way_slippage_bps =
            raydium_price_impact(trade_size_usd, pool_tvl_usd) * 10_000.0;
        let one_way_fee_bps =
            jupiter_effective_fee(trade_size_usd, 1, sc.base_fee_bps) * 10_000.0;
        let rt_cost_bps = round_trip_cost_bps(trade_size_usd, pool_tvl_usd, sc.base_fee_bps);
        let signal_strength_bps = signal.strength * 10_000.0;
        let net_edge_bps = signal_strength_bps - rt_cost_bps;

        // ── Candidate construction ────────────────────────────────────────
        let candidate = TradeCandidate {
            pool_address: signal.pool_address,
            pool_tvl_usd,
            estimated_entry_price: features.last_price,
            trade_size_usd,
            estimated_slippage_bps: one_way_slippage_bps,
            estimated_fee_bps: one_way_fee_bps,
            estimated_round_trip_cost_bps: rt_cost_bps,
            net_edge_bps,
            created_at_micros: now_micros,
            expires_at_micros: expires_at,
            signal,
        };

        // ── Filter chain ──────────────────────────────────────────────────
        if let Some((filter_name, reason)) =
            self.filter_chain.check_all(&candidate, &features, now_micros)
        {
            warn!(
                filter = filter_name,
                reason = reason,
                pool = ?candidate.pool_address,
                "trade rejected by filter chain"
            );
            let result = TradeResult {
                candidate,
                fill_price: 0.0,
                actual_slippage_bps: 0.0,
                fee_paid_bps: 0.0,
                pnl_bps: 0.0,
                rejected: true,
                reject_reason: Some(format!("[{filter_name}] {reason}")),
                executed_at_micros: now_micros,
            };
            self.push_history(result);
            return None;
        }

        // ── Paper simulation ──────────────────────────────────────────────
        let result =
            self.simulator
                .simulate_fill(&candidate, pool_tvl_usd, sc, now_micros);

        self.filter_chain.record_trade(candidate.pool_address, now_micros);

        debug!(
            signal_id = result.candidate.signal.signal_id,
            pnl_bps = result.pnl_bps,
            "paper fill executed"
        );

        self.push_history(result.clone());
        Some(result)
    }

    /// Returns an aggregated PnL summary over all recorded trades.
    pub fn pnl_summary(&self) -> PnlSummary {
        let history = self.trade_history.lock().unwrap();

        let total = history.len() as u64;
        let rejected = history.iter().filter(|r| r.rejected).count() as u64;

        let executed: Vec<&TradeResult> =
            history.iter().filter(|r| !r.rejected).collect();

        let winning = executed.iter().filter(|r| r.pnl_bps > 0.0).count() as u64;
        let losing = executed.len() as u64 - winning;

        let gross_pnl_bps: f64 = executed.iter().map(|r| r.pnl_bps).sum();
        let total_fees_bps: f64 = executed.iter().map(|r| r.fee_paid_bps * 2.0).sum();
        let net_pnl_bps = gross_pnl_bps - total_fees_bps;

        let win_rate = if executed.is_empty() {
            0.0
        } else {
            winning as f64 / executed.len() as f64
        };

        let avg_edge = if executed.is_empty() {
            0.0
        } else {
            executed
                .iter()
                .map(|r| r.candidate.net_edge_bps)
                .sum::<f64>()
                / executed.len() as f64
        };

        let rejection_rate = if total == 0 {
            0.0
        } else {
            rejected as f64 / total as f64
        };

        PnlSummary {
            total_trades: total,
            winning_trades: winning,
            losing_trades: losing,
            rejected_trades: rejected,
            gross_pnl_bps,
            total_fees_bps,
            net_pnl_bps,
            win_rate,
            avg_edge_captured_bps: avg_edge,
            rejection_rate,
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────

    fn push_history(&self, result: TradeResult) {
        let mut history = self.trade_history.lock().unwrap();
        if history.len() >= MAX_HISTORY {
            history.pop_front();
        }
        history.push_back(result);
    }
}
