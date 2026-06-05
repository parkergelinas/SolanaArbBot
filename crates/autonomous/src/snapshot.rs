//! Live runtime metrics exposed to the control API and dashboard.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuntimeSnapshot {
    pub running: bool,
    pub mode: String,
    pub scalp_trades: u64,
    pub arb_trades: u64,
    pub scalp_pnl_usd: f64,
    pub arb_pnl_usd: f64,
    pub net_pnl_usd: f64,
    pub winning_trades: u64,
    pub total_trades: u64,
    pub win_rate: f64,
    pub events_processed: u64,
    pub signals_emitted: u64,
    pub daily_loss_usd: f64,
    pub peak_equity_usd: f64,
    pub current_equity_usd: f64,
    pub risk_status: String,
    pub risk_state: String,
    pub trading_halted: bool,
    pub halt_reason: Option<String>,
    pub scalp_enabled: bool,
    pub arb_enabled: bool,
    pub runtime_mode: String,
    pub ingestion_mode: String,
    pub active_strategies: Vec<String>,
    pub signals_consumed: u64,
    pub signals_traded: u64,
    pub whale_signals_consumed: u64,
    pub momentum_signals_consumed: u64,
}

impl RuntimeSnapshot {
    pub fn refresh_derived(&mut self, capital_usd: f64) {
        self.net_pnl_usd = self.scalp_pnl_usd + self.arb_pnl_usd;
        self.total_trades = self.scalp_trades + self.arb_trades;
        self.current_equity_usd = capital_usd + self.net_pnl_usd;
        if self.current_equity_usd > self.peak_equity_usd {
            self.peak_equity_usd = self.current_equity_usd;
        }
        self.win_rate = if self.winning_trades == 0 {
            0.0
        } else {
            self.winning_trades as f64 / self.total_trades.max(1) as f64
        };
    }
}
