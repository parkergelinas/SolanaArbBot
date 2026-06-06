//! Paper execution simulator.
//!
//! `PaperSimulator` produces deterministic `TradeResult` values using the
//! microstructure models in `crate::microstructure`.  No wallet, no keypair,
//! no RPC call — all data comes from function parameters.

use config::ScalerConfig;
use signals::Direction;

use crate::candidate::{TradeCandidate, TradeResult};
use crate::microstructure::{jupiter_effective_fee, raydium_price_impact};

// ─────────────────────────────────────────────────────────────────────────────
// PaperSimulator
// ─────────────────────────────────────────────────────────────────────────────

/// Stateless paper-execution simulator.
///
/// Every call to [`simulate_fill`] is a pure function of its arguments,
/// ensuring deterministic PnL attribution regardless of call order.
pub struct PaperSimulator;

impl PaperSimulator {
    /// Simulate a fill for `candidate` and return the attributed `TradeResult`.
    ///
    /// # Slippage model
    ///
    /// Uses the Raydium CPMM price-impact formula with `pool_tvl_usd`.
    /// Long fills pay upward slippage; short fills pay downward slippage.
    ///
    /// # PnL formula
    ///
    /// ```text
    /// pnl_bps = signal_strength_bps
    ///           - actual_slippage_bps * 2   (entry + exit)
    ///           - fee_paid_bps * 2          (both legs)
    ///           - mev_haircut_bps
    /// ```
    pub fn simulate_fill(
        &self,
        candidate: &TradeCandidate,
        pool_tvl_usd: f64,
        cfg: &ScalerConfig,
        now_micros: u64,
    ) -> TradeResult {
        let one_way_impact = raydium_price_impact(candidate.trade_size_usd, pool_tvl_usd);
        let actual_slippage_bps = one_way_impact * 10_000.0;

        let fee_fraction = jupiter_effective_fee(candidate.trade_size_usd, 1, cfg.base_fee_bps);
        let fee_paid_bps = fee_fraction * 10_000.0;

        let fill_price = match candidate.signal.direction {
            Direction::Long => candidate.estimated_entry_price * (1.0 + one_way_impact),
            Direction::Short => candidate.estimated_entry_price * (1.0 - one_way_impact),
            Direction::Neutral => candidate.estimated_entry_price,
        };

        let signal_strength_bps = candidate.signal.strength * 10_000.0;
        let raw_pnl_bps = signal_strength_bps
            - actual_slippage_bps * 2.0
            - fee_paid_bps * 2.0
            - cfg.mev_haircut_bps;
        let tp_cap = cfg.take_profit_pct * 10_000.0;
        let sl_cap = cfg.stop_loss_pct * 10_000.0;
        let pnl_bps = raw_pnl_bps.clamp(-sl_cap, tp_cap);

        TradeResult {
            candidate: candidate.clone(),
            fill_price,
            actual_slippage_bps,
            fee_paid_bps,
            pnl_bps,
            rejected: false,
            reject_reason: None,
            executed_at_micros: now_micros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Pubkey;
    use config::ScalerConfig;
    use signals::{Direction, FeatureVector, SignalEvent, SignalType};

    fn make_candidate(direction: Direction, entry: f64, strength: f64, trade_usd: f64) -> TradeCandidate {
        let now = 1_000_000_000_u64;
        let signal = SignalEvent {
            signal_id: 1,
            timestamp_micros: now,
            pool_address: Pubkey::new([1; 32]),
            signal_type: SignalType::Momentum,
            strength,
            confidence: 0.9,
            direction,
            timeframe_secs: 60,
            feature_vector: FeatureVector {
                volume_short: 0.0,
                volume_long: 0.0,
                price_velocity: 0.0,
                liquidity_delta_pct: 0.0,
                whale_activity_score: 0.0,
                smart_money_score: 0.0,
                data_points: 0,
            },
            explanation: String::new(),
        };
        TradeCandidate {
            pool_address: signal.pool_address,
            pool_tvl_usd: 500_000.0,
            estimated_entry_price: entry,
            trade_size_usd: trade_usd,
            estimated_slippage_bps: 10.0,
            estimated_fee_bps: 5.0,
            estimated_round_trip_cost_bps: 30.0,
            net_edge_bps: strength * 10_000.0 - 30.0,
            created_at_micros: now,
            expires_at_micros: now + 30_000_000,
            signal,
        }
    }

    #[test]
    fn long_fill_price_is_above_entry() {
        let sim = PaperSimulator;
        let cfg = ScalerConfig::default();
        let candidate = make_candidate(Direction::Long, 100.0, 0.8, 1_000.0);
        let result = sim.simulate_fill(&candidate, 500_000.0, &cfg, 1_000_000_000);
        assert!(
            result.fill_price > 100.0,
            "long fill should be above entry, got {}",
            result.fill_price
        );
    }

    #[test]
    fn short_fill_price_is_below_entry() {
        let sim = PaperSimulator;
        let cfg = ScalerConfig::default();
        let candidate = make_candidate(Direction::Short, 100.0, 0.8, 1_000.0);
        let result = sim.simulate_fill(&candidate, 500_000.0, &cfg, 1_000_000_000);
        assert!(
            result.fill_price < 100.0,
            "short fill should be below entry, got {}",
            result.fill_price
        );
    }

    #[test]
    fn fill_is_not_rejected() {
        let sim = PaperSimulator;
        let cfg = ScalerConfig::default();
        let candidate = make_candidate(Direction::Long, 50.0, 0.75, 500.0);
        let result = sim.simulate_fill(&candidate, 1_000_000.0, &cfg, 1_000_000_000);
        assert!(!result.rejected);
        assert!(result.reject_reason.is_none());
    }

    #[test]
    fn take_profit_caps_positive_pnl() {
        let sim = PaperSimulator;
        let mut cfg = ScalerConfig::default();
        cfg.take_profit_pct = 0.01;
        let candidate = make_candidate(Direction::Long, 50.0, 0.99, 500.0);
        let result = sim.simulate_fill(&candidate, 1_000_000.0, &cfg, 1_000_000_000);
        assert!(result.pnl_bps <= 100.0 + f64::EPSILON);
    }

    #[test]
    fn stop_loss_caps_negative_pnl() {
        let sim = PaperSimulator;
        let mut cfg = ScalerConfig::default();
        cfg.stop_loss_pct = 0.005;
        cfg.mev_haircut_bps = 500.0;
        let candidate = make_candidate(Direction::Long, 50.0, 0.01, 500.0);
        let result = sim.simulate_fill(&candidate, 1_000_000.0, &cfg, 1_000_000_000);
        assert!(result.pnl_bps >= -50.0 - f64::EPSILON);
    }
}
