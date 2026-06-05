//! Coalesce intelligence messages into batched WS frames (25–50ms).

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;

use crossbeam_channel::Receiver;
use tokio::sync::broadcast;

use crate::contracts::{IntelligenceBatch, IntelligenceMessage};

pub fn batch_interval_ms() -> u64 {
    std::env::var("INTELLIGENCE_BATCH_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(33)
        .clamp(25, 50)
}

pub fn spawn_batcher(
    msg_rx: Receiver<IntelligenceMessage>,
    batch_tx: broadcast::Sender<IntelligenceBatch>,
) {
    let interval_ms = batch_interval_ms();
    let seq = Arc::new(AtomicU64::new(0));

    std::thread::spawn(move || {
        let mut buffer: Vec<IntelligenceMessage> = Vec::with_capacity(128);
        loop {
            match msg_rx.recv_timeout(Duration::from_millis(interval_ms)) {
                Ok(msg) => buffer.push(msg),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    if !buffer.is_empty() {
                        flush(&batch_tx, &seq, &mut buffer);
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
        if !buffer.is_empty() {
            flush(&batch_tx, &seq, &mut buffer);
        }
    });

    tracing::info!("intelligence broker batching every {interval_ms}ms");
}

fn flush(
    batch_tx: &broadcast::Sender<IntelligenceBatch>,
    seq: &AtomicU64,
    buffer: &mut Vec<IntelligenceMessage>,
) {
    let n = seq.fetch_add(1, Ordering::Relaxed);
    let frame = IntelligenceBatch::new(n, std::mem::take(buffer));
    let _ = batch_tx.send(frame);
}
