//! Coalesces `WSMessage` values into versioned `WSBatchFrame` ticks.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use std::time::Duration;

use crossbeam_channel::Receiver;
use tokio::sync::broadcast;

use crate::contracts::{WSBatchFrame, WSMessage};

pub fn batch_interval_ms() -> u64 {
    std::env::var("STREAM_BATCH_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(33)
        .clamp(25, 50)
}

/// Bridges crossbeam ingress → tokio broadcast batches.
pub fn spawn_batcher(
    msg_rx: Receiver<WSMessage>,
    batch_tx: broadcast::Sender<WSBatchFrame>,
) {
    let interval_ms = batch_interval_ms();
    let seq = Arc::new(AtomicU64::new(0));

    std::thread::spawn(move || {
        let mut buffer: Vec<WSMessage> = Vec::with_capacity(256);
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

    tracing::info!("stream broker batching every {interval_ms}ms");
}

fn flush(
    batch_tx: &broadcast::Sender<WSBatchFrame>,
    seq: &AtomicU64,
    buffer: &mut Vec<WSMessage>,
) {
    let n = seq.fetch_add(1, Ordering::Relaxed);
    let frame = WSBatchFrame::new(n, std::mem::take(buffer));
    let _ = batch_tx.send(frame);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_clamps() {
        std::env::set_var("STREAM_BATCH_MS", "10");
        assert_eq!(batch_interval_ms(), 25);
        std::env::remove_var("STREAM_BATCH_MS");
    }
}
