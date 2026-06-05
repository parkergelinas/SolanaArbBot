//! `SignalProcessor` trait — the single interface every detector must implement.
//!
//! Processors are **stateless**: all mutable state lives in [`FeatureStore`].
//! No processor may share mutable logic with another processor.

use config::SignalEngineConfig;

use crate::feature_store::ComputedFeatures;
use crate::types::{SignalEvent, SignalInput};

/// Interface for modular signal detectors.
///
/// # Contract
///
/// * Implementations **must** be `Send + Sync` (boxed into `Vec<Box<dyn …>>`).
/// * Implementations **must not** carry internal mutable state.
/// * All numeric thresholds **must** be read from `cfg`; no hardcoded values.
/// * Returning an empty `Vec` means "no signal detected" — this is not an error.
pub trait SignalProcessor: Send + Sync {
    /// Attempt to generate zero or more signals from the given input + features.
    ///
    /// `now_micros` is the event wall-clock time; passing it explicitly keeps
    /// every detector fully deterministic and unit-testable without wall-clock.
    fn process(
        &self,
        input: &SignalInput,
        features: &ComputedFeatures,
        cfg: &SignalEngineConfig,
        now_micros: u64,
    ) -> Vec<SignalEvent>;

    /// Human-readable name used in traces.
    fn name(&self) -> &'static str;
}
