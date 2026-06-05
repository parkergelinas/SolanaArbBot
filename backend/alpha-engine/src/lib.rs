//! Alpha signal automation — fusion, scoring, queue, emission.

#![forbid(unsafe_code)]

pub mod config;
pub mod emitter;
pub mod fusion;
pub mod inputs;
pub mod latency;
pub mod pipeline;
pub mod queue;
pub mod rules;
pub mod scoring;
pub mod types;

pub use config::AlphaConfig;
pub use fusion::FusionEngine;
pub use pipeline::AlphaPipeline;
pub use types::{AlphaSignal, FusedFeatureVector, TradeSignal};
