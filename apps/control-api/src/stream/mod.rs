//! Stream engine — coalesces domain events into batched WebSocket frames.

mod batcher;

pub use batcher::{spawn_batcher, WsBatchFrame};
