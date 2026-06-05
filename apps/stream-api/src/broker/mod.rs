//! WebSocket broker — batches outbound messages every 25–50 ms.

mod batch;

pub use batch::{batch_interval_ms, spawn_batcher};
