//! System-wide configuration for the Solana market analysis pipeline.
//!
//! [`SystemConfig`] is the single top-level configuration struct.  All
//! values use plain Rust types (no crate-specific imports) so that `config`
//! can sit at the base of the dependency graph and be used by every crate,
//! including `apps/worker`.

#![forbid(unsafe_code)]

use std::time::Duration;

/// Top-level system configuration.
///
/// Construct with [`SystemConfig::default()`] and override individual fields
/// as needed.  Each domain crate converts the relevant sub-section into its
/// own config type.
#[derive(Clone, Debug)]
pub struct SystemConfig {
    // ── engine ─────────────────────────────────────────────────────────────
    /// Maximum events the engine processes before its run-loop exits.
    pub max_pipeline_events: usize,
    /// Subscriber receive timeout inside the engine run-loop.
    pub pipeline_event_timeout: Duration,
    /// Notional USD amount used as the starting input for route simulations.
    pub simulation_initial_amount: f64,

    // ── event bus ──────────────────────────────────────────────────────────
    /// Bounded channel capacity per subscriber.
    pub event_channel_capacity: usize,

    // ── ingestion ──────────────────────────────────────────────────────────
    /// Maximum number of events emitted by the placeholder ingestion source.
    pub ingestion_event_limit: usize,
    /// Yield cadence for the ingestion async task (0 = never yield).
    pub ingestion_yield_every: usize,

    // ── routing ────────────────────────────────────────────────────────────
    /// Maximum route depth in hops (must be 2–3).
    pub routing_max_depth: usize,
    /// Minimum edge liquidity allowed in candidate routes.
    pub routing_min_liquidity: f64,
    /// Per-hop depth penalty in basis points.
    pub routing_depth_penalty_bps: u64,

    // ── execution simulation ───────────────────────────────────────────────
    /// Minimum edge liquidity required for simulation.
    pub execution_min_liquidity: f64,
    /// Maximum input-to-liquidity ratio per hop.
    pub execution_max_input_ratio: f64,
    /// CLMM slippage multiplier for the simplified tick model.
    pub execution_clmm_slippage_multiplier: f64,

    // ── route-level risk ───────────────────────────────────────────────────
    /// Minimum per-edge liquidity for risk approval.
    pub risk_min_liquidity: f64,
    /// Maximum tolerated route slippage (fraction, e.g. 0.05 = 5 %).
    pub risk_max_slippage: f64,
    /// Maximum route depth accepted by the risk engine.
    pub risk_max_depth: usize,
    /// Minimum route price product (below this triggers rejection).
    pub risk_min_price_product: f64,
    /// Maximum route price product (above this triggers rejection).
    pub risk_max_price_product: f64,

    // ── portfolio risk ─────────────────────────────────────────────────────
    /// Capital under management in USD.
    pub portfolio_capital_usd: f64,
    /// Maximum daily loss as a fraction of capital.
    pub portfolio_daily_max_loss_pct: f64,
    /// Maximum monthly loss as a fraction of capital.
    pub portfolio_monthly_max_loss_pct: f64,
    /// Maximum drawdown from peak equity.
    pub portfolio_max_drawdown_pct: f64,
    /// Cumulative loss fraction that triggers a permanent halt.
    pub portfolio_total_max_loss_pct: f64,
    /// Duration to pause after breaching the daily loss limit.
    pub portfolio_daily_pause: Duration,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            max_pipeline_events: 1_024,
            pipeline_event_timeout: Duration::from_millis(100),
            simulation_initial_amount: 10.0,

            event_channel_capacity: 4_096,

            ingestion_event_limit: 1_024,
            ingestion_yield_every: 256,

            routing_max_depth: 3,
            routing_min_liquidity: 0.0,
            routing_depth_penalty_bps: 50,

            execution_min_liquidity: 1.0,
            execution_max_input_ratio: 0.25,
            execution_clmm_slippage_multiplier: 0.35,

            risk_min_liquidity: 1_000.0,
            risk_max_slippage: 0.05,
            risk_max_depth: 3,
            risk_min_price_product: 0.2,
            risk_max_price_product: 5.0,

            portfolio_capital_usd: 1_000.0,
            portfolio_daily_max_loss_pct: 0.05,
            portfolio_monthly_max_loss_pct: 0.15,
            portfolio_max_drawdown_pct: 0.25,
            portfolio_total_max_loss_pct: 0.40,
            portfolio_daily_pause: Duration::from_secs(3_600),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SystemConfig;

    #[test]
    fn default_config_has_sensible_bounds() {
        let cfg = SystemConfig::default();

        assert!(cfg.max_pipeline_events > 0);
        assert!(cfg.simulation_initial_amount > 0.0);
        assert!(cfg.routing_max_depth >= 2 && cfg.routing_max_depth <= 3);
        assert!(cfg.risk_max_slippage > 0.0 && cfg.risk_max_slippage < 1.0);
        assert!(cfg.portfolio_capital_usd > 0.0);
        assert!(cfg.portfolio_total_max_loss_pct < 1.0);
    }
}
