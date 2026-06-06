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
        use crate::jupiter_swap::execute_swap_blocking;
        use config::ArbitrageConfig;

        let release = || {
            let _ = self.exposure_release_tx.send(
                intent.size_lamports.saturating_mul(100) / 1_000_000,
            );
        };

        let wallet = match &self.wallet {
            Some(w) => w,
            None => {
                tracing::error!(
                    pool_idx = intent.pool_idx,
                    slot = intent.slot,
                    "Jito submission attempted without a loaded wallet keypair — \
                     set features.dry_run=false and SOLANA_ARB_WALLET_KEY"
                );
                release();
                return;
            }
        };

        let opportunity_id = format!("hotpath-{}-{}", intent.pool_idx, intent.slot);
        let cfg = ArbitrageConfig::default();
        let submitter = JitoSubmitter::new(&cfg);

        // Step 1: Fetch recent blockhash for tip tx.
        let blockhash = match crate::jito::fetch_blockhash_blocking(&self.rpc_endpoint) {
            Ok(bh) => bh,
            Err(e) => {
                tracing::error!(error = %e, pool_idx = intent.pool_idx,
                    "blockhash fetch failed; dropping intent");
                release();
                return;
            }
        };

        // Step 2: Get a real signed swap tx from Jupiter.
        let (input_mint, output_mint) = pool_mints_for_idx(intent.pool_idx);
        let swap_bytes = match execute_swap_blocking(
            input_mint,
            output_mint,
            intent.size_lamports,
            50, // 50 bps slippage
            wallet,
        ) {
            Ok(result) => {
                let raw = base64_decode_simple(&result.signed_tx_b64);
                tracing::debug!(
                    pool_idx = intent.pool_idx,
                    out_amount = result.out_amount,
                    tx_len = raw.len(),
                    "jupiter swap tx built"
                );
                raw
            }
            Err(e) => {
                tracing::error!(error = %e, pool_idx = intent.pool_idx,
                    "Jupiter swap build failed; dropping intent");
                release();
                return;
            }
        };

        // Step 3: Build tip tx + inject swap tx → submit bundle.
        let req = BundleRequest {
            opportunity_id: opportunity_id.clone(),
            tip_lamports: intent.tip_lamports,
            priority_fee_lamports: 100_000,
            amount_in_lamports: intent.size_lamports,
            route_hops: 2,
        };
        let payer = wallet.pubkey();
        let mut bundle = build_bundle_signed(
            &submitter,
            &req,
            payer.as_bytes(),
            &blockhash,
            |msg| wallet.sign_message(msg).unwrap_or([0u8; 64]),
        );
        bundle.swap_tx = swap_bytes;

        match crate::jito::submit_bundle_blocking_raw(&submitter, bundle, &opportunity_id) {
            Ok(r) => {
                tracing::info!(
                    pool_idx = intent.pool_idx,
                    bundle_id = %r.bundle_id,
                    "Jito bundle submitted"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    pool_idx = intent.pool_idx,
                    "Jito bundle submission failed"
                );
            }
        }

        // Always release exposure regardless of submission outcome.
        release();
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

/// Map pool_idx to (input_mint, output_mint) for Jupiter quote calls.
///
/// Even pool indices are Raydium or Orca pools where we sell SOL for USDC.
/// Odd indices represent the return leg (USDC → SOL) or alternative pairs.
/// This mapping mirrors the pool registry in `ingestion::pools`.
fn pool_mints_for_idx(pool_idx: u16) -> (&'static str, &'static str) {
    // Well-known mainnet mint addresses.
    const WSOL:  &str = "So11111111111111111111111111111111111111112";
    const USDC:  &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    const USDT:  &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

    match pool_idx {
        0 | 4 => (WSOL, USDC), // Raydium/CLMM SOL/USDC
        1     => (WSOL, USDC), // Orca SOL/USDC
        2     => (WSOL, USDT), // Raydium SOL/USDT
        3     => (WSOL, USDT), // Orca SOL/USDT
        _     => (WSOL, USDC), // fallback
    }
}

/// Decode a base64 string to bytes using the standard alphabet.
/// Returns an empty Vec if decoding fails.
fn base64_decode_simple(s: &str) -> Vec<u8> {
    // Use the base64 crate already in scope via the `base64` dep in execution/Cargo.toml.
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .unwrap_or_default()
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
