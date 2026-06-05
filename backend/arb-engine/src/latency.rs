//! Detection-path latency instrumentation.

use std::time::Instant;
use tracing::warn;

#[derive(Clone, Debug, Default)]
pub struct DetectionBudget {
    pub normalize_ms: f64,
    pub pool_update_ms: f64,
    pub spread_ms: f64,
    pub signal_ms: f64,
    pub total_ms: f64,
}

pub struct DetectionTracker {
    started: Instant,
    pub budget: DetectionBudget,
}

impl DetectionTracker {
    pub fn start() -> Self {
        Self {
            started: Instant::now(),
            budget: DetectionBudget::default(),
        }
    }

    pub fn record(&mut self, field: &str, started: Instant) {
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        match field {
            "normalize" => self.budget.normalize_ms = ms,
            "pool" => self.budget.pool_update_ms = ms,
            "spread" => self.budget.spread_ms = ms,
            "signal" => self.budget.signal_ms = ms,
            _ => {}
        }
    }

    pub fn finish(&mut self) -> &DetectionBudget {
        self.budget.total_ms = self.started.elapsed().as_secs_f64() * 1000.0;
        if self.budget.total_ms > 50.0 {
            warn!(
                normalize_ms = self.budget.normalize_ms,
                pool_ms = self.budget.pool_update_ms,
                spread_ms = self.budget.spread_ms,
                signal_ms = self.budget.signal_ms,
                total_ms = self.budget.total_ms,
                "arb detection exceeded 50ms budget"
            );
        }
        &self.budget
    }
}
