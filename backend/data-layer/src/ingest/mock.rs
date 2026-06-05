//! High-frequency mock RawUpdate stream for local dev (<150ms e2e testing).

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crossbeam_channel::Sender;
use tracing::info;

use super::adapter::IngestionAdapter;
use crate::types::RawUpdate;

const DEXES: &[&str] = &["raydium", "orca", "jupiter"];
const TOKENS: &[&str] = &[
    "So11111111111111111111111111111111111111112",
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
];
const WHALE_WALLETS: &[&str] = &[
    "whale_alpha_7xK9m2pQ",
    "smart_gamma_2jH5c8fL",
];

pub struct MockAdapter {
    pub interval_ms: u64,
}

impl MockAdapter {
    pub fn from_env() -> Self {
        let interval_ms = std::env::var("MOCK_INGEST_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(40);
        Self { interval_ms }
    }
}

impl IngestionAdapter for MockAdapter {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn spawn(self: Box<Self>, tx: Sender<RawUpdate>) -> anyhow::Result<()> {
        let stop = Arc::new(AtomicBool::new(false));
        let interval = self.interval_ms;
        let mut seq: u64 = 0;
        let mut slot: u64 = 300_000_000;

        std::thread::spawn(move || {
            info!("mock ingestion streaming every {interval}ms");
            while !stop.load(Ordering::Relaxed) {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                let dex = DEXES[(seq as usize) % DEXES.len()];
                let token = TOKENS[(seq as usize) % TOKENS.len()];
                let is_whale = seq % 9 == 0;
                let wallet = if is_whale {
                    WHALE_WALLETS[(seq as usize / 9) as usize % WHALE_WALLETS.len()]
                } else {
                    "retail_wallet"
                };
                let amount_sol = if is_whale {
                    50.0 + (seq % 200) as f64
                } else {
                    0.5 + (seq % 20) as f64 * 0.1
                };

                let signature = format!("mock_sig_{seq:016x}");
                let log = format!(
                    "Program log: {dex} swap wallet={wallet} token={token} amount_sol={amount_sol:.4}"
                );

                let update = RawUpdate {
                    slot,
                    signature,
                    logs: vec![log],
                    accounts: vec![token.to_string()],
                    timestamp: ts,
                };

                if tx.send(update).is_err() {
                    break;
                }

                seq = seq.wrapping_add(1);
                slot = slot.wrapping_add(1);
                std::thread::sleep(std::time::Duration::from_millis(interval));
            }
        });

        Ok(())
    }
}
