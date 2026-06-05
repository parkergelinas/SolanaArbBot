use std::time::Duration;

#[derive(Clone, Debug)]
pub struct RiskConfig {
    pub capital_usd: f64,
    pub daily_max_loss_pct: f64,
    pub monthly_max_loss_pct: f64,
    pub max_drawdown_pct: f64,
    pub total_max_loss_pct: f64,
    pub daily_pause: Duration,
}

impl RiskConfig {
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

    /// Records a PnL delta (positive for profit, negative for loss).
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
            // profits reduce total loss magnitude
            self.total_loss += pnl;
        }
    }
}
