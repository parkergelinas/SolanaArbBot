//! Execution routing — Jito bundles (preferred) with direct RPC fallback.
//!
//! Network I/O lives exclusively in the cold-path thread.  The hot loop only
//! produces `ExecutionIntent` values and pushes them to an SPSC queue.

use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender, TrySendError};
use wallet::WalletKeypair;

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
    /// Returns the notional exposure (USD ×100) to the hot path after each submission (item 3).
    exposure_release_tx: crossbeam_channel::Sender<u64>,
    /// Solana RPC endpoint for blockhash fetch and direct-RPC sendTransaction.
    rpc_endpoint: String,
    /// Live signing keypair; `None` in paper mode.
    wallet: Option<Arc<WalletKeypair>>,
}

impl ColdPathExecutor {
    #[must_use]
    pub fn new(
        router: ExecutionRouter,
        rx: Receiver<ExecutionIntent>,
        exposure_release_tx: crossbeam_channel::Sender<u64>,
        rpc_endpoint: String,
        wallet: Option<Arc<WalletKeypair>>,
    ) -> Self {
        Self {
            router,
            rx,
            exposure_release_tx,
            rpc_endpoint,
            wallet,
        }
    }

    /// Blocking receive loop — call from a non-hot thread only.
    pub fn run_loop(&self) {
        while let Ok(intent) = self.rx.recv() {
            let choice = self.router.choose(&intent);
            match choice {
                RouteChoice::JitoBundle => {
                    self.submit_jito(&intent);
                }
                RouteChoice::DirectRpc => {
                    self.submit_rpc(&intent);
                }
                RouteChoice::PaperSimulated => {
                    // Paper fill — release exposure immediately so the counter stays accurate.
                    let _ = self.exposure_release_tx.send(intent.size_lamports);
                }
            }
        }
    }

    fn submit_jito(&self, intent: &ExecutionIntent) {
        use crate::jito::{BundleRequest, JitoSubmitter, build_bundle_signed};
        use config::ArbitrageConfig;

        let cfg = ArbitrageConfig::default();
        let submitter = JitoSubmitter::new(&cfg);

        match &self.wallet {
            Some(wallet) => {
                let req = BundleRequest {
                    opportunity_id: format!("hotpath-{}-{}", intent.pool_idx, intent.slot),
                    tip_lamports: intent.tip_lamports,
                    priority_fee_lamports: 100_000,
                    amount_in_lamports: intent.size_lamports,
                    route_hops: 2,
                };
                // Fetch recent blockhash, build and sign the tip tx, then submit.
                match crate::jito::fetch_blockhash_blocking(&self.rpc_endpoint) {
                    Ok(blockhash) => {
                        let payer = wallet.pubkey();
                        let bundle = build_bundle_signed(
                            &submitter,
                            &req,
                            payer.as_bytes(),
                            &blockhash,
                            |msg| {
                                wallet.sign_message(msg).unwrap_or([0u8; 64])
                            },
                        );
                        let _ = crate::jito::submit_bundle_blocking_raw(&submitter, bundle, &req.opportunity_id);
                    }
                    Err(e) => {
                        tracing::error!(
                            pool_idx = intent.pool_idx,
                            slot = intent.slot,
                            error = %e,
                            "failed to fetch blockhash for Jito bundle; intent dropped"
                        );
                    }
                }
            }
            None => {
                tracing::error!(
                    pool_idx = intent.pool_idx,
                    slot = intent.slot,
                    "Jito submission attempted without a loaded wallet keypair; intent dropped. \
                     Ensure features.dry_run=false and a live keypair is configured."
                );
            }
        }

        // Always release exposure so the hot-path counter does not permanently lock (item 3).
        let _ = self.exposure_release_tx.send(
            intent.size_lamports.saturating_mul(100) / 1_000_000, // rough USD×100 estimate
        );
    }

    /// Direct RPC submission is not yet implemented.
    ///
    /// This path requires building a fully signed Solana transaction and calling
    /// `sendTransaction` on the configured RPC endpoint.  Until that is wired,
    /// all intents routed here are logged and dropped so they do not silently
    /// vanish (item 2).
    fn submit_rpc(&self, intent: &ExecutionIntent) {
        tracing::error!(
            pool_idx = intent.pool_idx,
            slot = intent.slot,
            size_lamports = intent.size_lamports,
            rpc_endpoint = %self.rpc_endpoint,
            "direct RPC execution path is not implemented — intent dropped. \
             Implement wallet signing + sendTransaction to enable this path."
        );
        // Release exposure so the hot-path counter does not permanently lock (item 3).
        let _ = self.exposure_release_tx.send(
            intent.size_lamports.saturating_mul(100) / 1_000_000,
        );
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
    // Ceiling division: round the cost UP so min_out is never weakened by truncation.
    let cost_bps = (impact.saturating_add(route.round_trip_fee_bps())) as u128;
    let cost = (trade_size as u128 * cost_bps + 9_999) / 10_000;
    let min_out = trade_size.saturating_sub(cost as u64);

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
