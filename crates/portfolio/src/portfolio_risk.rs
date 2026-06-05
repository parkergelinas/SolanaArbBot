//! Portfolio-level risk state machine.
//!
//! Tracks equity, drawdown, and cumulative losses across the session.
//! Evaluates configured limits and returns the action the system should take.

use std::time::Duration;

/// Configuration thresholds for portfolio-level risk controls.
#[derive(Clone, Debug)]
pub struct RiskConfig {
    /// Total capital under management in USD.
    pub capital_usd: f64,
    /// Maximum daily loss as a fraction of capital (e.g. 0.05 = 5 %).
    pub daily_max_loss_pct: f64,
    /// Maximum monthly loss as a fraction of capital (e.g. 0.15 = 15 %).
    pub monthly_max_loss_pct: f64,
    /// Maximum drawdown from peak equity as a fraction (e.g. 0.25 = 25 %).
    pub max_drawdown_pct: f64,
    /// Maximum total loss as a fraction of capital triggering permanent halt.
    pub total_max_loss_pct: f64,
    /// Duration to pause trading after breaching the daily limit.
    pub daily_pause: Duration,
}

impl RiskConfig {
    /// Returns a default risk configuration for the given capital amount.
    pub fn default_with_capital(capital: f64) -> Self {
        Self {
            capital_usd: capital,
            daily_max_loss_pct: 0.05,
            monthly_max_loss_pct: 0.15,
            max_drawdown_pct: 0.25,
            total_max_loss_pct: 0.40,
            daily_pause: Duration::from_secs(60 * 60),
        }
    }
}

/// Live portfolio risk state.
#[derive(Clone, Debug)]
pub struct RiskState {
    pub capital_usd: f64,
    pub peak_equity: f64,
    pub current_equity: f64,
    pub cumulative_daily_loss: f64,
    pub cumulative_monthly_loss: f64,
    pub total_loss: f64,
}

impl RiskState {
    /// Creates an initial risk state from starting capital.
    pub fn new(capital: f64) -> Self {
        Self {
            capital_usd: capital,
            peak_equity: capital,
            current_equity: capital,
            cumulative_daily_loss: 0.0,
            cumulative_monthly_loss: 0.0,
            total_loss: 0.0,
        }
    }

    /// Records a PnL delta (positive = profit, negative = loss).
    pub fn record_pnl(&mut self, pnl: f64) {
        self.current_equity += pnl;
        if self.current_equity > self.peak_equity {
            self.peak_equity = self.current_equity;
        }

        if pnl < 0.0 {
            self.cumulative_daily_loss += pnl;
            self.cumulative_monthly_loss += pnl;
            self.total_loss += pnl;
        } else {
            self.total_loss += pnl;
        }
    }
}

/// Action the system should take after evaluating risk limits.
#[derive(Debug, PartialEq, Eq)]
pub enum RiskAction {
    /// No limit breached — continue operating.
    Ok,
    /// Pause trading for the specified duration.
    Pause(Duration),
    /// Pause trading for the specified number of days.
    PauseDays(u64),
    /// Permanently halt — requires manual intervention to restart.
    PermanentHalt,
}

/// Evaluates the configured limits and returns the required action.
pub fn evaluate_limits(state: &RiskState, cfg: &RiskConfig) -> RiskAction {
    if state.total_loss <= -cfg.total_max_loss_pct * cfg.capital_usd {
        return RiskAction::PermanentHalt;
    }

    let drawdown = if state.peak_equity > 0.0 {
        (state.peak_equity - state.current_equity) / state.peak_equity
    } else {
        0.0
    };

    if drawdown >= cfg.max_drawdown_pct {
        return RiskAction::PauseDays(7);
    }

    if state.cumulative_monthly_loss <= -cfg.monthly_max_loss_pct * cfg.capital_usd {
        return RiskAction::PauseDays(30);
    }

    if state.cumulative_daily_loss <= -cfg.daily_max_loss_pct * cfg.capital_usd {
        return RiskAction::Pause(cfg.daily_pause);
    }

    RiskAction::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_limit_triggers_pause() {
        let cfg = RiskConfig::default_with_capital(250.0);
        let mut st = RiskState::new(cfg.capital_usd);
        st.record_pnl(-12.50);
        match evaluate_limits(&st, &cfg) {
            RiskAction::Pause(d) => assert_eq!(d, Duration::from_secs(60 * 60)),
            _ => panic!("expected pause"),
        }
    }

    #[test]
    fn monthly_limit_triggers_30d() {
        let cfg = RiskConfig::default_with_capital(250.0);
        let mut st = RiskState::new(cfg.capital_usd);
        st.cumulative_monthly_loss = -37.50;
        match evaluate_limits(&st, &cfg) {
            RiskAction::PauseDays(d) => assert_eq!(d, 30),
            _ => panic!("expected 30-day pause"),
        }
    }

    #[test]
    fn drawdown_triggers_7d() {
        let cfg = RiskConfig::default_with_capital(250.0);
        let mut st = RiskState::new(cfg.capital_usd);
        st.peak_equity = 300.0;
        st.current_equity = 225.0;
        match evaluate_limits(&st, &cfg) {
            RiskAction::PauseDays(d) => assert_eq!(d, 7),
            _ => panic!("expected 7-day pause"),
        }
    }

    #[test]
    fn total_loss_triggers_permanent_halt() {
        let cfg = RiskConfig::default_with_capital(250.0);
        let mut st = RiskState::new(cfg.capital_usd);
        st.total_loss = -100.0;
        assert_eq!(evaluate_limits(&st, &cfg), RiskAction::PermanentHalt);
    }
}
