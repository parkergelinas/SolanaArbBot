//! Yellowstone Geyser gRPC adapter for sniper pool-creation events.
//!
//! Full tonic proto client is gated behind the `yellowstone` feature (future).
//! Until then, a stub keeps the pipeline alive when `YELLOWSTONE_ENDPOINT` is set.

use crossbeam_channel::Sender;
use tracing::{info, warn};

use crate::sniper_ingest::{parse_pool_creation_from_logs, PoolCreationEvent};

/// Spawn a background Yellowstone consumer for sniper pool-creation logs.
pub fn spawn_sniper_geyser(endpoint: String, tx: Sender<PoolCreationEvent>) {
    #[cfg(feature = "yellowstone")]
    {
        return spawn_yellowstone_sniper(endpoint, tx);
    }

    #[cfg(not(feature = "yellowstone"))]
    {
        warn!(
            endpoint = %endpoint,
            "YELLOWSTONE_ENDPOINT set but crate built without `yellowstone` feature — \
             rebuild with: cargo build -p ingestion --features yellowstone"
        );
        std::thread::spawn(move || {
            info!(endpoint = %endpoint, "yellowstone sniper stub running");
            loop {
                let _ = tx.send(PoolCreationEvent {
                    source: crate::sniper_ingest::PoolCreationSource::PumpFun,
                    token_mint: "stub_mint_yellowstone_pending".to_owned(),
                    pool_address: "stub_pool".to_owned(),
                    creator_wallet: "stub_creator".to_owned(),
                    initial_liquidity_sol: 0.0,
                    signature: "yellowstone_stub".to_owned(),
                    slot: 0,
                });
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        });
    }
}

/// Map a raw geyser log batch to a pool-creation event (shared by stub + future client).
pub fn map_geyser_logs(
    logs: &[String],
    signature: &str,
    slot: u64,
) -> Option<PoolCreationEvent> {
    parse_pool_creation_from_logs(logs, signature, slot)
}

#[cfg(feature = "yellowstone")]
fn spawn_yellowstone_sniper(endpoint: String, tx: Sender<PoolCreationEvent>) {
    use std::time::Duration;

    std::thread::spawn(move || {
        info!(endpoint = %endpoint, "yellowstone sniper consumer starting (proto stub)");
        loop {
            // Production: tonic SubscribeService → map updates → parse_pool_creation_from_logs
            std::thread::sleep(Duration::from_secs(5));
            let _ = &tx;
        }
    });
}
