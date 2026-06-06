//! Production configuration backbone for the Solana market-analysis pipeline.
//!
//! [`SystemConfig`] is the **single source of truth** for all runtime
//! configuration across the workspace.  It is loaded once at startup,
//! wrapped in [`Arc`] via [`ConfigHandle`], and shared immutably across every
//! pipeline stage.
//!
//! # Loading order (last write wins)
//!
//! 1. Built-in defaults (`SystemConfig::default()`)
//! 2. TOML config file (`SOLANA_ARB_CONFIG` env var → `./config.toml`)
//! 3. Environment variables (`SOLANA_ARB_<SECTION>__<FIELD>`)
//!
//! # Quick start
//!
//! ```rust,no_run
//! use config::ConfigHandle;
//! use std::sync::Arc;
//!
//! // Load from file + env vars:
//! let cfg = ConfigHandle::load().expect("config load failed");
//!
//! // Access fields directly through Deref:
//! println!("capital: ${}", cfg.capital_usd());
//! println!("dry run:  {}", cfg.features.dry_run);
//!
//! // Share across threads cheaply (just increments the Arc refcount):
//! let arc: Arc<_> = cfg.arc();
//! ```
//!
//! # Environment variable reference
//!
//! ```text
//! SOLANA_ARB_RISK__CAPITAL_USD=50000
//! SOLANA_ARB_FEATURES__DRY_RUN=false
//! SOLANA_ARB_EXECUTION__MAX_SLIPPAGE_BPS=30
//! SOLANA_ARB_MONITORING__LOG_LEVEL=debug
//! SOLANA_ARB_RPC__ENDPOINTS=https://a.example.com,https://b.example.com
//! ```
//!
//! See `config.example.toml` in the workspace root for a fully-annotated
//! configuration file.

#![forbid(unsafe_code)]

pub mod loader;
pub mod schema;

pub use loader::ConfigHandle;
pub use schema::{
    DataSourcesConfig, ExecutionConfig, FeatureFlags, IngestionConfig, MonitoringConfig,
    OrchestratorConfig, PipelineConfig, PortfolioConfig, RetryConfig, RiskConfig, RpcConfig,
    ScalerConfig, HotPathConfig, SignalEngineConfig, StrategyConfig, SystemConfig, WalletConfig,
    WebSocketConfig,
};

use std::path::PathBuf;
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur during configuration loading or validation.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The config file could not be read from the filesystem.
    #[error("failed to read config file '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The TOML content could not be parsed.
    #[error("failed to parse config file: {0}")]
    Parse(String),

    /// A required config file path does not exist.
    #[error("configuration file not found: {}", .0.display())]
    FileNotFound(PathBuf),

    /// A loaded configuration failed domain validation.
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

/// Convenience alias for Results within the `config` crate.
pub type ConfigResult<T> = Result<T, ConfigError>;
