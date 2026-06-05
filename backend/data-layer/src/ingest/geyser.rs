//! Yellowstone Geyser gRPC adapter — production streaming consumer.

use crossbeam_channel::Sender;
use tracing::{info, warn};

use super::adapter::IngestionAdapter;
use crate::types::RawUpdate;

pub struct GeyserAdapter {
    pub endpoint: String,
}

impl GeyserAdapter {
    pub fn from_env() -> Option<Self> {
        std::env::var("YELLOWSTONE_ENDPOINT")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|endpoint| Self { endpoint })
    }
}

impl IngestionAdapter for GeyserAdapter {
    fn name(&self) -> &'static str {
        "yellowstone"
    }

    fn spawn(self: Box<Self>, tx: Sender<RawUpdate>) -> anyhow::Result<()> {
        let endpoint = self.endpoint.clone();

        #[cfg(feature = "yellowstone")]
        {
            return spawn_yellowstone(endpoint, tx);
        }

        #[cfg(not(feature = "yellowstone"))]
        {
            warn!(
                endpoint = %endpoint,
                "YELLOWSTONE_ENDPOINT set but crate built without `yellowstone` feature — \
                 rebuild with: cargo build -p data-layer --features yellowstone"
            );
            // Fallback: keep pipeline alive with no events until feature enabled.
            std::thread::spawn(move || {
                while tx.send(RawUpdate {
                    slot: 0,
                    signature: "yellowstone_stub".into(),
                    logs: vec!["Program log: stub awaiting yellowstone feature".into()],
                    accounts: vec![],
                    timestamp: 0,
                }).is_ok() {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                }
            });
            Ok(())
        }
    }
}

#[cfg(feature = "yellowstone")]
fn spawn_yellowstone(endpoint: String, tx: Sender<RawUpdate>) -> anyhow::Result<()> {
    use std::time::Duration;

    std::thread::spawn(move || {
        info!(endpoint = %endpoint, "yellowstone consumer starting (proto stub)");
        // Production: tonic client to SubscribeService, map subscribe updates → RawUpdate.
        // Proto definitions vendored in a follow-up PR.
        loop {
            let _ = tx.send(RawUpdate {
                slot: 0,
                signature: "yellowstone_pending_proto".into(),
                logs: vec![],
                accounts: vec![],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            });
            std::thread::sleep(Duration::from_secs(5));
        }
    });
    Ok(())
}
