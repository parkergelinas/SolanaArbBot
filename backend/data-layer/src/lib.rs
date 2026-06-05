//! Real-time Solana data layer — ingest, parse, normalize, enrich, route.
//!
//! Pipeline:
//! Yellowstone/Mock → RawUpdate → Parser → SwapEvent → Enrichment → Router

#![forbid(unsafe_code)]

pub mod enrich;
pub mod ingest;
pub mod normalize;
pub mod parser;
pub mod pipeline;
pub mod router;
pub mod types;

pub use pipeline::{spawn_pipeline, PipelineConfig, PipelineHandles};
pub use types::*;
