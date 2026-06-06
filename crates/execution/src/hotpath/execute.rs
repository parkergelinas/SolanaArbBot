//! Execution routing — Jito bundles (preferred) with direct RPC fallback.
//!
//! Network I/O lives exclusively in the cold-path thread.  The hot loop only
//! produces `ExecutionIntent` values and pushes them to an SPSC queue.

use crossbeam_channel::{Receiver, Sender, TrySendError};

use super::precompute::PrecomputeTable;
use super::types::ExecutionIntent;

/// Routing decision for cold-path submission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteChoice {
    JitoBundle,
    DirectRpc,
    PaperSimulated,
}

/// Selects submission path based on precomputed preferences.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionRouter {
    prefer_jito: bool,
    allow_direct_rpc: bool,
    paper_mode: bool,
}

impl ExecutionRouter {
    #[must_use]
    pub fn from_table(table: &PrecomputeTable) -> Self {
        Self {
            prefer_jito: table.prefer_jito,
            allow_direct_rpc: table.allow_direct_rpc,
            paper_mode: table.paper_mode,
        }
    }

    #[inline]
    pub fn choose(&self, intent: &ExecutionIntent) -> RouteChoice {
        if self.paper_mode {
            return RouteChoice::PaperSimulated;
        }
        if intent.use_jito && self.prefer_jito {
            RouteChoice::JitoBundle
        } else if self.allow_direct_rpc {
            RouteChoice::DirectRpc
        } else {
            RouteChoice::PaperSimulated
        }
    }
}

/// SPSC queue connecting hot loop → cold-path I/O thread.
pub struct ExecutionQueue {
    tx: Sender<ExecutionIntent>,
    rx: Receiver<ExecutionIntent>,
}

impl ExecutionQueue {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(capacity);
        Self { tx, rx }
    }

    /// Non-blocking enqueue from the hot loop.
    #[inline]
    pub fn try_enqueue(&self, intent: ExecutionIntent) -> bool {
        !matches!(self.tx.try_send(intent), Err(TrySendError::Full(_)))
    }

    pub fn sender(&self) -> Sender<ExecutionIntent> {
        self.tx.clone()
    }

    pub fn receiver(&self) -> Receiver<ExecutionIntent> {
        self.rx.clone()
    }
}

/// Cold-path executor — runs in a dedicated thread, may block on I/O.
pub struct ColdPathExecutor {
    router: ExecutionRouter,
    rx: Receiver<ExecutionIntent>,
}

impl ColdPathExecutor {
    #[must_use]
    pub fn new(router: ExecutionRouter, rx: Receiver<ExecutionIntent>) -> Self {
        Self { router, rx }
    }

    /// Blocking receive loop — call from a non-hot thread only.
    pub fn run_loop(&self) {
        while let Ok(intent) = self.rx.recv() {
            let choice = self.router.choose(&intent);
            match choice {
                RouteChoice::JitoBundle => {
                    Self::submit_jito(&intent);
                }
                RouteChoice::DirectRpc => {
                    Self::submit_rpc(&intent);
                }
                RouteChoice::PaperSimulated => {
                    // Paper fill — no network, no logging in production hot path.
                    let _ = intent;
                }
            }
        }
    }

    fn submit_jito(intent: &ExecutionIntent) {
        use crate::jito::{BundleRequest, JitoSubmitter};
        use config::ArbitrageConfig;

        let submitter = JitoSubmitter::new(&ArbitrageConfig::default());
        let req = BundleRequest {
            opportunity_id: format!("hotpath-{}-{}", intent.pool_idx, intent.slot),
            tip_lamports: intent.tip_lamports,
            priority_fee_lamports: 100_000,
            amount_in_lamports: intent.size_lamports,
            route_hops: 2,
        };
        let _ = crate::jito::submit_bundle_blocking(&submitter, req);
        let _ = intent;
    }

    fn submit_rpc(intent: &ExecutionIntent) {
        // Placeholder: direct RPC sendTransaction with pre-built tx bytes.
        let _ = intent;
    }
}

/// Build an execution intent from an approved signal.
#[inline]
pub fn build_intent(
    signal: &super::types::HotSignal,
    pool: &super::state::PoolSlot,
    table: &PrecomputeTable,
) -> ExecutionIntent {
    let route = table.route(signal.route_idx);
    let trade_size = table.thresholds.default_trade_lamports;
    let impact = pool.estimate_impact_bps(trade_size);
    let min_out = trade_size.saturating_sub(
        (trade_size as u128 * (impact + route.round_trip_fee_bps()) as u128 / 10_000) as u64,
    );

    ExecutionIntent {
        pool_idx: signal.pool_idx,
        route_idx: signal.route_idx,
        slot: signal.slot,
        size_lamports: trade_size,
        min_out,
        edge_bps: signal.edge_bps,
        use_jito: table.prefer_jito,
        tip_lamports: table.max_jito_tip_lamports.min(50_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_mode_routes_to_simulated() {
        let router = ExecutionRouter {
            prefer_jito: true,
            allow_direct_rpc: true,
            paper_mode: true,
        };
        let intent = ExecutionIntent {
            pool_idx: 0,
            route_idx: 0,
            slot: 1,
            size_lamports: 1000,
            min_out: 990,
            edge_bps: 30,
            use_jito: true,
            tip_lamports: 1000,
        };
        assert_eq!(router.choose(&intent), RouteChoice::PaperSimulated);
    }

    #[test]
    fn queue_try_enqueue_succeeds() {
        let q = ExecutionQueue::new(4);
        let intent = ExecutionIntent {
            pool_idx: 0,
            route_idx: 0,
            slot: 1,
            size_lamports: 1000,
            min_out: 990,
            edge_bps: 30,
            use_jito: false,
            tip_lamports: 0,
        };
        assert!(q.try_enqueue(intent));
    }
}
