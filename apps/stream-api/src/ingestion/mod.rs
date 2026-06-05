//! Solana event ingestion — mock today; RPC / Geyser adapters plug in here.

mod mock;

pub use mock::{spawn_mock_ingestion, IngestionHandle};

use crossbeam_channel::Sender;

use crate::contracts::SwapEvent;

/// Trait for future RPC / Geyser sources.
pub trait SwapSource: Send + Sync {
    fn poll(&mut self) -> Option<SwapEvent>;
}

/// Fan-out swap events into the market engine ingress channel.
pub fn forward_swap(tx: &Sender<SwapEvent>, swap: SwapEvent) {
    let _ = tx.send(swap);
}
