//! Stub subscriber — maps intelligence whale alerts → trade signals.

use crossbeam_channel::Sender;
use tracing::info;

use crate::signals::TradeSignal;

const SOL: &str = "So11111111111111111111111111111111111111112";
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Demo signal generator for local integration testing.
pub fn spawn_demo_signal_feed(tx: Sender<TradeSignal>, interval_ms: u64) {
    std::thread::spawn(move || {
        let mut n: u64 = 0;
        loop {
            n += 1;
            let signal = TradeSignal::new(
                "demo_wallet",
                SOL,
                USDC,
                0.75 + (n % 3) as f64 * 0.05,
                12.0 + (n % 5) as f64,
                25.0,
                if n % 2 == 0 { "scalp" } else { "arb" },
            );
            if tx.send(signal).is_err() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
        }
    });
    info!("demo signal feed started (interval {interval_ms}ms)");
}

/// Future: subscribe to intelligence-api internal channel and map whale alerts.
pub fn spawn_intelligence_stub(_tx: Sender<TradeSignal>) {
    info!("intelligence signal bridge stub — wire to data-layer whale alerts in production");
}
