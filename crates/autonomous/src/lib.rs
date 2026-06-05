//! Autonomous dual-strategy runtime: scalping + DEX-to-DEX arbitrage.
//!
//! Paper-only execution with orchestrator risk gates and daily loss halts.

#![forbid(unsafe_code)]

pub mod ingestion;
pub mod runtime;
pub mod snapshot;

pub use runtime::{spawn_autonomous_runtime, AutonomousCallbacks};
pub use snapshot::RuntimeSnapshot;

/// Strategy config tuned for scalping + DEX-to-DEX paper operation.
pub fn tuned_config() -> config::SystemConfig {
    let mut cfg = config::SystemConfig::default();
    cfg.risk.capital_usd = 10_000.0;
    cfg.risk.daily_loss_limit_pct = 0.03;
    cfg.risk.max_drawdown_pct = 0.15;
    cfg.scalper.min_edge_bps = 25.0;
    cfg.scalper.trade_cooldown_secs = 15;
    cfg.scalper.min_position_usd = 200.0;
    cfg.scalper.max_position_usd = 350.0;
    cfg.scalper.max_trades_per_hour_global = 120;
    cfg.scalper.volatility_floor = 0.0;
    cfg.scalper.min_volume_5m_usd = 50.0;
    cfg.scalper.mev_haircut_bps = 5.0;
    cfg.signal_engine.signal_min_strength = 0.05;
    cfg.signal_engine.signal_min_confidence = 0.05;
    cfg.signal_engine.cooldown_secs = 5;
    cfg.execution.min_profit_threshold_usd = 0.10;
    cfg.execution.simulation_initial_amount_usd = 200.0;
    cfg.features.dry_run = true;
    cfg
}
