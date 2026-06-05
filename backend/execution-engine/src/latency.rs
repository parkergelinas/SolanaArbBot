//! Decision-path latency instrumentation.

use std::time::Instant;
use tracing::warn;

#[derive(Clone, Debug, Default)]
pub struct LatencyBudget {
    pub strategy_ms: f64,
    pub risk_ms: f64,
    pub quote_ms: f64,
    pub swap_build_ms: f64,
    pub total_ms: f64,
}

pub struct LatencyTracker {
    started: Instant,
    pub budget: LatencyBudget,
}

impl LatencyTracker {
    pub fn start() -> Self {
        Self {
            started: Instant::now(),
            budget: LatencyBudget::default(),
        }
    }

    pub fn record_strategy(&mut self, started: Instant) {
        self.budget.strategy_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn record_risk(&mut self, started: Instant) {
        self.budget.risk_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn record_quote(&mut self, started: Instant) {
        self.budget.quote_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn record_swap_build(&mut self, started: Instant) {
        self.budget.swap_build_ms = started.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn finish(&mut self) -> &LatencyBudget {
        self.budget.total_ms = self.started.elapsed().as_secs_f64() * 1000.0;
        if self.budget.total_ms > 100.0 {
            warn!(
                strategy_ms = self.budget.strategy_ms,
                risk_ms = self.budget.risk_ms,
                quote_ms = self.budget.quote_ms,
                swap_build_ms = self.budget.swap_build_ms,
                total_ms = self.budget.total_ms,
                "execution decision exceeded 100ms budget"
            );
        }
        &self.budget
    }
}
