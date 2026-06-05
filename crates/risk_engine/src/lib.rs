pub mod state;
pub mod enforcer;

pub use state::{RiskConfig, RiskState};
pub use enforcer::{RiskAction, evaluate_limits};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn daily_limit_triggers_pause() {
        let cfg = RiskConfig::default_with_capital(250.0);
        let mut st = RiskState::new(cfg.capital_usd);

        // lose 12.50 (5% of 250)
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

        // simulate peak 300 -> current equity 225 (25% drawdown)
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

        st.total_loss = -100.0; // 40% of 250
        let action = evaluate_limits(&st, &cfg);

        match action {
            RiskAction::PermanentHalt => {}
            _ => panic!("expected permanent halt"),
        }
    }
}
