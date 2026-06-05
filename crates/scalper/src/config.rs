//! Re-export `ScalerConfig` from the base `config` crate.
//!
//! `ScalerConfig` is defined in `config::schema` (the workspace base crate) so
//! that it can be embedded in `SystemConfig` without creating a dependency
//! cycle.  This module gives callers of the `scalper` crate a single, stable
//! import path.

// ScalerConfig will be re-exported here once the type is added to the
// workspace `config` crate.  Placeholder to avoid a broken import.
// pub use ::config::ScalerConfig;
