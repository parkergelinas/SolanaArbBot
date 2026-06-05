//! Mock Solana swap stream — simulates whale wallets and DEX activity.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crossbeam_channel::Sender;

use crate::contracts::{Dex, FlowSide, TransactionEvent, SCHEMA_VERSION};

const MINTS: &[(&str, f64)] = &[
    ("So11111111111111111111111111111111111111112", 145.0),
    ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 1.0),
    ("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", 1.0),
    ("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", 0.85),
];

/// Known whale wallets in mock mode (deterministic labels).
const WHALE_WALLETS: &[&str] = &[
    "whale_alpha_7xK9m2pQ",
    "whale_beta_4nR8v1wT",
    "smart_gamma_2jH5c8fL",
];

pub struct IngestionHandle {
    stop: Arc<AtomicBool>,
}

impl IngestionHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub fn spawn_mock_ingestion(
    tx: Sender<TransactionEvent>,
    interval_ms: u64,
) -> IngestionHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let mut slot: u64 = 290_000_000;
    let mut seq: u64 = 0;

    std::thread::spawn(move || {
        while !stop_flag.load(Ordering::Relaxed) {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let dex = match seq % 3 {
                0 => Dex::Raydium,
                1 => Dex::Orca,
                _ => Dex::Jupiter,
            };

            let (token_in, price_in) = MINTS[(seq as usize) % MINTS.len()];
            let (token_out, price_out) = MINTS[((seq as usize) + 1) % MINTS.len()];

            let is_whale_wallet = seq % 7 == 0 || seq % 11 == 0;
            let wallet = if is_whale_wallet {
                WHALE_WALLETS[(seq as usize / 7) as usize % WHALE_WALLETS.len()].to_string()
            } else {
                format!("retail_{:06x}", seq % 0xffffff)
            };

            let base_amount = if is_whale_wallet {
                50_000_000u64 + (seq % 200_000_000)
            } else {
                500_000u64 + (seq % 5_000_000)
            };

            let notional = (base_amount as f64) * price_in / 1_000_000_000.0;
            let amount_out = ((notional / price_out) * 1_000_000.0) as u64;
            let side = if seq % 2 == 0 {
                FlowSide::Buy
            } else {
                FlowSide::Sell
            };

            let event = TransactionEvent {
                v: SCHEMA_VERSION,
                signature: format!("intel_sig_{seq:016x}"),
                dex,
                wallet,
                token_in: token_in.to_string(),
                token_out: token_out.to_string(),
                amount_in: base_amount.to_string(),
                amount_out: amount_out.to_string(),
                notional_usd: notional,
                side,
                slot,
                timestamp_ms: ts,
            };

            let _ = tx.send(event);
            seq = seq.wrapping_add(1);
            slot = slot.wrapping_add(1);
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
        }
    });

    IngestionHandle { stop }
}
