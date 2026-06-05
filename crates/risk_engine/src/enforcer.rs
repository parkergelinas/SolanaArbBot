use crate::state::{RiskConfig, RiskState};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum RiskAction {
    Ok,
    Pause(Duration),
    PauseDays(u64),
    PermanentHalt,
}

/// Evaluates the configured limits and returns the action to take.
pub fn evaluate_limits(state: &RiskState, cfg: &RiskConfig) -> RiskAction {
    // Total loss (absolute negative) check -> permanent halt
    if state.total_loss <= -cfg.total_max_loss_pct * cfg.capital_usd {
        return RiskAction::PermanentHalt;
    }

    // Drawdown check: drop from peak
    let drawdown = if state.peak_equity > 0.0 {
        (state.peak_equity - state.current_equity) / state.peak_equity
    } else {
        0.0
    };

    if drawdown >= cfg.max_drawdown_pct {
        return RiskAction::PauseDays(7);
    }

    // Monthly loss
    if state.cumulative_monthly_loss <= -cfg.monthly_max_loss_pct * cfg.capital_usd {
        return RiskAction::PauseDays(30);
    }

    // Daily loss
    if state.cumulative_daily_loss <= -cfg.daily_max_loss_pct * cfg.capital_usd {
        return RiskAction::Pause(cfg.daily_pause);
    }

    RiskAction::Ok
}
