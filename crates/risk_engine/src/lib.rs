//! Backward-compatible re-export of [`portfolio::portfolio_risk`].
//!
//! All existing consumers of `risk_engine` continue to work without
//! modification.  New code should import from `portfolio` directly.
//!
//! The legacy source files `state.rs` and `enforcer.rs` are retained in the
//! repository for reference but are no longer compiled as part of this crate.

#![forbid(unsafe_code)]

pub use portfolio::{evaluate_limits, RiskAction, RiskConfig, RiskState};

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{evaluate_limits, RiskAction, RiskConfig, RiskState};

    #[test]
    fn daily_limit_triggers_pause() {
        let cfg = RiskConfig::default_with_capital(250.0);
        let mut st = RiskState::new(cfg.capital_usd);

        st.record_pnl(-12.50);
        let action = evaluate_limits(&st, &cfg);

        match action {
            RiskAction::Pause(d) => assert_eq!(d, Duration::from_secs(60 * 60)),
            _ => panic!("expected pause"),
        }
    }

    #[test]
    fn monthly_limit_triggers_30d() {
        let cfg = RiskConfig::default_with_capital(250.0);
        let mut st = RiskState::new(cfg.capital_usd);

        st.cumulative_monthly_loss = -37.50;
        let action = evaluate_limits(&st, &cfg);

        match action {
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
        let action = evaluate_limits(&st, &cfg);

        match action {
            RiskAction::PauseDays(d) => assert_eq!(d, 7),
            _ => panic!("expected 7-day pause"),
        }
    }

    #[test]
    fn total_loss_triggers_permanent_halt() {
        let cfg = RiskConfig::default_with_capital(250.0);
        let mut st = RiskState::new(cfg.capital_usd);

        st.total_loss = -100.0;
        let action = evaluate_limits(&st, &cfg);

        match action {
            RiskAction::PermanentHalt => {}
            _ => panic!("expected permanent halt"),
        }
    }
}
