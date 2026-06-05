//! Batches `WsEvent` values and publishes `WsBatchFrame` on a fixed interval (25–50 ms).

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use serde::{Deserialize, Serialize};
use tokio::{
    sync::{broadcast, mpsc},
    time::{self, Duration},
};

use crate::events::WsEvent;

/// WebSocket wire frame — one JSON message per flush tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsBatchFrame {
    #[serde(rename = "type")]
    pub frame_type: &'static str,
    pub seq: u64,
    pub ts_micros: u64,
    pub events: Vec<WsEvent>,
}

impl WsBatchFrame {
    pub fn new(seq: u64, events: Vec<WsEvent>) -> Self {
        let ts_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);
        Self {
            frame_type: "batch",
            seq,
            ts_micros,
            events,
        }
    }
}

/// Reads `STREAM_BATCH_MS` (default 33), clamped to [25, 50].
pub fn batch_interval_ms() -> u64 {
    std::env::var("STREAM_BATCH_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(33)
        .clamp(25, 50)
}

/// Spawns the batching task. Returns the ingress sender used by `AppState::emit`.
pub fn spawn_batcher(
    batch_tx: broadcast::Sender<WsBatchFrame>,
) -> mpsc::UnboundedSender<WsEvent> {
    let (ingress_tx, mut ingress_rx) = mpsc::unbounded_channel::<WsEvent>();
    let seq = Arc::new(AtomicU64::new(0));
    let interval_ms = batch_interval_ms();

    tokio::spawn(async move {
        let mut buffer: Vec<WsEvent> = Vec::with_capacity(64);
        let mut tick = time::interval(Duration::from_millis(interval_ms));
        tick.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                maybe = ingress_rx.recv() => {
                    match maybe {
                        Some(ev) => buffer.push(ev),
                        None => break,
                    }
                }
                _ = tick.tick() => {
                    if buffer.is_empty() {
                        continue;
                    }
                    let n = seq.fetch_add(1, Ordering::Relaxed);
                    let frame = WsBatchFrame::new(n, std::mem::take(&mut buffer));
                    let _ = batch_tx.send(frame);
                }
            }
        }

        // Drain remaining events on shutdown.
        while let Ok(ev) = ingress_rx.try_recv() {
            buffer.push(ev);
        }
        if !buffer.is_empty() {
            let n = seq.fetch_add(1, Ordering::Relaxed);
            let _ = batch_tx.send(WsBatchFrame::new(n, buffer));
        }
    });

    tracing::info!("stream batcher started — flush every {interval_ms}ms");
    ingress_tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_interval_defaults_and_clamps() {
        std::env::remove_var("STREAM_BATCH_MS");
        assert_eq!(batch_interval_ms(), 33);

        std::env::set_var("STREAM_BATCH_MS", "10");
        assert_eq!(batch_interval_ms(), 25);

        std::env::set_var("STREAM_BATCH_MS", "99");
        assert_eq!(batch_interval_ms(), 50);

        std::env::remove_var("STREAM_BATCH_MS");
    }

    #[test]
    fn batch_frame_serialises_with_type_batch() {
        let frame = WsBatchFrame::new(1, vec![]);
        let json = serde_json::to_string(&frame).expect("serialize");
        assert!(json.contains("\"type\":\"batch\""));
        assert!(json.contains("\"seq\":1"));
    }
}
