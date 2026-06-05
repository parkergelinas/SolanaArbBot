//! Alpha pipeline latency instrumentation.

use std::time::Instant;
use tracing::warn;

#[derive(Clone, Debug, Default)]
pub struct AlphaLatencyBudget {
    pub fusion_ms: f64,
    pub scoring_ms: f64,
    pub gate_ms: f64,
    pub queue_ms: f64,
    pub total_ms: f64,
}

pub struct AlphaLatencyTracker {
    started: Instant,
    pub budget: AlphaLatencyBudget,
}

impl AlphaLatencyTracker {
    pub fn start() -> Self {
        Self {
            started: Instant::now(),
            budget: AlphaLatencyBudget::default(),
        }
    }

    pub fn record_fusion(&mut self, t: Instant) {
        self.budget.fusion_ms = t.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn record_scoring(&mut self, t: Instant) {
        self.budget.scoring_ms = t.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn record_gate(&mut self, t: Instant) {
        self.budget.gate_ms = t.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn record_queue(&mut self, t: Instant) {
        self.budget.queue_ms = t.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn finish(&mut self) -> &AlphaLatencyBudget {
        self.budget.total_ms = self.started.elapsed().as_secs_f64() * 1000.0;
        if self.budget.total_ms > 50.0 {
            warn!(
                fusion_ms = self.budget.fusion_ms,
                scoring_ms = self.budget.scoring_ms,
                gate_ms = self.budget.gate_ms,
                queue_ms = self.budget.queue_ms,
                total_ms = self.budget.total_ms,
                "alpha pipeline exceeded 50ms budget"
            );
        }
        &self.budget
    }
}
