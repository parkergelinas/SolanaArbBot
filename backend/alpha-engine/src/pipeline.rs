//! Alpha pipeline — fusion → score → gate → queue per token event.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tracing::{debug, trace};

use crate::config::AlphaConfig;
use crate::emitter::SignalEmitter;
use crate::fusion::FusionEngine;
use crate::latency::AlphaLatencyTracker;
use crate::queue::PriorityQueue;
use crate::rules::SignalGate;
use crate::scoring::score_to_alpha;
use crate::types::unix_ms;

pub struct AlphaPipeline {
    config: Arc<AlphaConfig>,
    fusion: Arc<FusionEngine>,
    gate: SignalGate,
    queue: Mutex<PriorityQueue>,
    emitter: SignalEmitter,
}

impl AlphaPipeline {
    pub fn new(
        config: Arc<AlphaConfig>,
        fusion: Arc<FusionEngine>,
        emitter: SignalEmitter,
    ) -> Self {
        let queue = PriorityQueue::new(config.queue_max, config.queue_ttl_ms);
        Self {
            gate: SignalGate::new(&config),
            config,
            fusion,
            queue: Mutex::new(queue),
            emitter,
        }
    }

    /// Process one token after fusion update — incremental, no full rescan.
    pub fn on_token_updated(&self, token: &str, wallet: &str) {
        let mut lat = AlphaLatencyTracker::start();
        let t0 = Instant::now();

        let fused = match self.fusion.get(token) {
            Some(f) => f,
            None => return,
        };
        lat.record_fusion(t0);

        let t1 = Instant::now();
        let alpha = match score_to_alpha(&fused, &self.config, wallet) {
            Some(a) => a,
            None => return,
        };
        lat.record_scoring(t1);

        let t2 = Instant::now();
        let trade = match self.gate.evaluate(&fused, &alpha) {
            Ok(t) => t,
            Err(e) => {
                trace!(?e, token, "signal gate rejected");
                return;
            }
        };
        lat.record_gate(t2);

        let t3 = Instant::now();
        if let Ok(mut q) = self.queue.lock() {
            q.push(alpha);
        }
        lat.record_queue(t3);

        let budget = lat.finish();
        debug!(
            token,
            total_ms = budget.total_ms,
            queue_depth = self.queue_depth(),
            "alpha pipeline cycle"
        );

        let _ = self.emitter.emit(trade);
    }

    pub fn drain_tick(&self) -> usize {
        let now = unix_ms();
        let signals = if let Ok(mut q) = self.queue.lock() {
            q.drain_top_n(self.config.drain_per_tick, now)
        } else {
            return 0;
        };

        let mut emitted = 0;
        for alpha in signals {
            if let Some(fused) = self.fusion.get(&alpha.token_out) {
                if let Ok(trade) = self.gate.evaluate(&fused, &alpha) {
                    if self.emitter.emit(trade) {
                        emitted += 1;
                    }
                }
            }
        }
        emitted
    }

    pub fn queue_depth(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }
}
