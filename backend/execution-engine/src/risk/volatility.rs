use crate::signals::TradeSignal;

pub trait VolatilityFilter: Send + Sync {
    fn allows(&self, signal: &TradeSignal) -> bool;
}

pub struct NoOpVolatilityFilter;

impl VolatilityFilter for NoOpVolatilityFilter {
    fn allows(&self, _signal: &TradeSignal) -> bool {
        true
    }
}

/// Stub for future token volatility gating.
pub struct TokenVolatilityFilter {
    pub max_volatility: f64,
}

impl VolatilityFilter for TokenVolatilityFilter {
    fn allows(&self, signal: &TradeSignal) -> bool {
        signal.expected_edge <= self.max_volatility * 100.0
    }
}
