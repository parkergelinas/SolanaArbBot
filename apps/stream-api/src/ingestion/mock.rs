//! Mock swap generator — simulates Raydium / Orca / Jupiter activity.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crossbeam_channel::Sender;

use crate::contracts::{Dex, SwapEvent};

const MINTS: &[(&str, f64)] = &[
    ("So11111111111111111111111111111111111111112", 145.0),
    ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", 1.0),
    ("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", 1.0),
];

pub struct IngestionHandle {
    stop: Arc<AtomicBool>,
}

impl IngestionHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Spawns a background task emitting swaps at `interval_ms` until stopped.
pub fn spawn_mock_ingestion(
    tx: Sender<SwapEvent>,
    interval_ms: u64,
) -> IngestionHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let mut slot: u64 = 280_000_000;
    let mut seq: u64 = 0;

    std::thread::spawn(move || {
        while !stop_flag.load(Ordering::Relaxed) {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let dex = match seq % 4 {
                0 => Dex::Raydium,
                1 => Dex::Orca,
                2 => Dex::Jupiter,
                _ => Dex::Pump,
            };

            let (token_in, price_in) = MINTS[(seq as usize) % MINTS.len()];
            let (token_out, price_out) = MINTS[((seq as usize) + 1) % MINTS.len()];
            let amount_in = 1_000_000u64 + (seq % 50_000);
            let notional = (amount_in as f64) * price_in / 1_000_000_000.0;
            let amount_out = ((notional / price_out) * 1_000_000.0) as u64;

            let swap = SwapEvent::new_v1(
                format!("mock_sig_{seq:012x}"),
                dex,
                token_in,
                token_out,
                amount_in.to_string(),
                amount_out.to_string(),
                format!("mock_wallet_{:04x}", seq % 0xffff),
                slot,
                ts,
            );

            let _ = tx.send(swap);
            seq = seq.wrapping_add(1);
            slot = slot.wrapping_add(1);

            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
        }
    });

    IngestionHandle { stop }
}
