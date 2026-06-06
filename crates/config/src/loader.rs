//! Configuration loading: defaults → TOML file → environment variable overrides.
//!
//! ## Layer order (last write wins)
//!
//! 1. **Built-in defaults** — every field in [`SystemConfig`] implements [`Default`].
//! 2. **Config file** — TOML at path from `SOLANA_ARB_CONFIG` env var, or
//!    `./config.toml`.  Missing files are silently ignored for [`ConfigHandle::load`].
//! 3. **Environment variables** — `SOLANA_ARB_<SECTION>__<FIELD>` (see table below).
//!
//! ## Environment variable naming
//!
//! | Env var | Config path |
//! |---|---|
//! | `SOLANA_ARB_PORTFOLIO__CAPITAL_USD` | `portfolio.capital_usd` |
//! | `SOLANA_ARB_EXECUTION__MAX_SLIPPAGE_BPS` | `execution.max_slippage_bps` |
//! | `SOLANA_ARB_FEATURES__DRY_RUN` | `features.dry_run` |
//! | `SOLANA_ARB_MONITORING__LOG_LEVEL` | `monitoring.log_level` |
//! | `SOLANA_ARB_RPC__ENDPOINTS` | `rpc.endpoints` (comma-separated) |
//! | `SOLANA_ARB_DATA_SOURCES__HELIUS_API_KEY` | `data_sources.helius_api_key` |
//!
//! Boolean values accept: `1` / `true` / `yes` / `on`  →  `true`
//!                        `0` / `false` / `no` / `off` →  `false`

use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{ConfigError, ConfigResult, SystemConfig};

// ─────────────────────────────────────────────────────────────────────────────
// ConfigHandle
// ─────────────────────────────────────────────────────────────────────────────

/// Thread-safe, immutable handle to the loaded [`SystemConfig`].
///
/// Cloning is cheap — it only increments the reference count on the inner
/// [`Arc`].  The wrapped config is immutable after construction.
#[derive(Clone, Debug)]
pub struct ConfigHandle(Arc<SystemConfig>);

impl ConfigHandle {
    /// Loads configuration from the standard layered sources.
    ///
    /// File path resolved from `SOLANA_ARB_CONFIG` env var or `./config.toml`.
    /// Missing files are silently skipped; env var overrides are always applied.
    pub fn load() -> ConfigResult<Self> {
        let path = std::env::var("SOLANA_ARB_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("config.toml"));
        Self::load_with_path_optional(path)
    }

    /// Loads from an explicit TOML file path.
    ///
    /// Returns [`ConfigError::FileNotFound`] if the file does not exist.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.to_path_buf()));
        }
        Self::load_with_path_optional(path)
    }

    /// Loads configuration from a raw TOML string.
    ///
    /// Useful for unit tests and in-process config injection.  Environment
    /// variable overrides are NOT applied.  Partial sections merge over defaults.
    pub fn from_toml_str(toml_str: &str) -> ConfigResult<Self> {
        let mut config = SystemConfig::default();
        merge_toml_into_config(&mut config, toml_str)?;
        config.validate().map_err(ConfigError::Invalid)?;
        Ok(Self(Arc::new(config)))
    }

    /// Constructs a handle directly from a pre-built [`SystemConfig`].
    ///
    /// Skips file loading and env-var application.  The caller is responsible
    /// for validation.
    pub fn from_config(config: SystemConfig) -> Self {
        Self(Arc::new(config))
    }

    /// Returns a clone of the inner [`Arc`] for shared ownership.
    pub fn arc(&self) -> Arc<SystemConfig> {
        Arc::clone(&self.0)
    }

    fn load_with_path_optional<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let path = path.as_ref();

        let mut config = SystemConfig::default();
        if path.exists() {
            let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;
            merge_toml_into_config(&mut config, &content)?;
        }

        apply_env_overrides(&mut config);

        config.validate().map_err(ConfigError::Invalid)?;
        Ok(Self(Arc::new(config)))
    }
}

impl Deref for ConfigHandle {
    type Target = SystemConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TOML merge (partial sections overlay defaults)
// ─────────────────────────────────────────────────────────────────────────────

fn merge_toml_into_config(config: &mut SystemConfig, toml_str: &str) -> ConfigResult<()> {
    let base_value = toml::Value::try_from(&*config)
        .map_err(|e| ConfigError::Parse(format!("serialize defaults: {e}")))?;
    let overlay: toml::Value =
        toml::from_str(toml_str).map_err(|e| ConfigError::Parse(e.to_string()))?;
    let merged = deep_merge_toml(base_value, overlay);
    let merged_str =
        toml::to_string(&merged).map_err(|e| ConfigError::Parse(format!("serialize merged: {e}")))?;
    *config =
        toml::from_str(&merged_str).map_err(|e| ConfigError::Parse(e.to_string()))?;
    Ok(())
}

fn deep_merge_toml(base: toml::Value, overlay: toml::Value) -> toml::Value {
    match (base, overlay) {
        (toml::Value::Table(mut base_table), toml::Value::Table(overlay_table)) => {
            for (key, overlay_value) in overlay_table {
                let base_entry = base_table
                    .remove(&key)
                    .unwrap_or(toml::Value::Table(toml::value::Table::new()));
                base_table.insert(key, deep_merge_toml(base_entry, overlay_value));
            }
            toml::Value::Table(base_table)
        }
        (_, overlay) => overlay,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Environment variable overrides
// ─────────────────────────────────────────────────────────────────────────────

/// Applies `SOLANA_ARB_*` environment variables on top of `config`.
///
/// Unknown or unparsable values are silently ignored — the previous value is
/// kept, which is always a valid default.
pub(crate) fn apply_env_overrides(config: &mut SystemConfig) {
    macro_rules! env_scalar {
        ($env:expr, $field:expr, $ty:ty) => {
            if let Ok(raw) = std::env::var($env) {
                if let Ok(v) = raw.trim().parse::<$ty>() {
                    $field = v;
                }
            }
        };
    }

    macro_rules! env_bool {
        ($env:expr, $field:expr) => {
            if let Ok(raw) = std::env::var($env) {
                match raw.trim().to_lowercase().as_str() {
                    "1" | "true" | "yes" | "on" => $field = true,
                    "0" | "false" | "no" | "off" => $field = false,
                    _ => {}
                }
            }
        };
    }

    macro_rules! env_string {
        ($env:expr, $field:expr) => {
            if let Ok(raw) = std::env::var($env) {
                let trimmed = raw.trim().to_owned();
                if !trimmed.is_empty() {
                    $field = trimmed;
                }
            }
        };
    }

    macro_rules! env_string_vec {
        ($env:expr, $field:expr) => {
            if let Ok(raw) = std::env::var($env) {
                let items: Vec<String> = raw
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect();
                if !items.is_empty() {
                    $field = items;
                }
            }
        };
    }

    macro_rules! env_option_string {
        ($env:expr, $field:expr) => {
            if let Ok(raw) = std::env::var($env) {
                let trimmed = raw.trim().to_owned();
                if trimmed.is_empty() {
                    $field = None;
                } else {
                    $field = Some(trimmed);
                }
            }
        };
    }

    // ── [rpc] ─────────────────────────────────────────────────────────────
    env_string_vec!("SOLANA_ARB_RPC__ENDPOINTS", config.rpc.endpoints);
    env_scalar!("SOLANA_ARB_RPC__TIMEOUT_MS", config.rpc.timeout_ms, u64);
    env_scalar!("SOLANA_ARB_RPC__MAX_RETRIES", config.rpc.max_retries, u32);
    env_string!("SOLANA_ARB_RPC__COMMITMENT", config.rpc.commitment);
    env_scalar!(
        "SOLANA_ARB_RPC__MAX_CONCURRENT_REQUESTS",
        config.rpc.max_concurrent_requests,
        usize
    );

    // ── [websocket] ───────────────────────────────────────────────────────
    env_string_vec!("SOLANA_ARB_WEBSOCKET__ENDPOINTS", config.websocket.endpoints);
    env_scalar!(
        "SOLANA_ARB_WEBSOCKET__RECONNECT_INTERVAL_MS",
        config.websocket.reconnect_interval_ms,
        u64
    );
    env_scalar!(
        "SOLANA_ARB_WEBSOCKET__PING_INTERVAL_MS",
        config.websocket.ping_interval_ms,
        u64
    );
    env_scalar!(
        "SOLANA_ARB_WEBSOCKET__MAX_RECONNECT_ATTEMPTS",
        config.websocket.max_reconnect_attempts,
        u32
    );
    env_scalar!(
        "SOLANA_ARB_WEBSOCKET__RECEIVE_TIMEOUT_MS",
        config.websocket.receive_timeout_ms,
        u64
    );

    // ── [retry] ───────────────────────────────────────────────────────────
    env_scalar!(
        "SOLANA_ARB_RETRY__INITIAL_INTERVAL_MS",
        config.retry.initial_interval_ms,
        u64
    );
    env_scalar!(
        "SOLANA_ARB_RETRY__MAX_INTERVAL_MS",
        config.retry.max_interval_ms,
        u64
    );
    env_scalar!("SOLANA_ARB_RETRY__MULTIPLIER", config.retry.multiplier, f64);
    env_scalar!("SOLANA_ARB_RETRY__MAX_RETRIES", config.retry.max_retries, u32);
    env_bool!("SOLANA_ARB_RETRY__JITTER", config.retry.jitter);

    // ── [execution] ───────────────────────────────────────────────────────
    env_scalar!(
        "SOLANA_ARB_EXECUTION__MAX_SLIPPAGE_BPS",
        config.execution.max_slippage_bps,
        u64
    );
    env_scalar!(
        "SOLANA_ARB_EXECUTION__PRIORITY_FEE_LAMPORTS",
        config.execution.priority_fee_lamports,
        u64
    );
    env_scalar!(
        "SOLANA_ARB_EXECUTION__MIN_PROFIT_THRESHOLD_USD",
        config.execution.min_profit_threshold_usd,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_EXECUTION__SIMULATION_INITIAL_AMOUNT_USD",
        config.execution.simulation_initial_amount_usd,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_EXECUTION__MAX_INPUT_RATIO",
        config.execution.max_input_ratio,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_EXECUTION__CLMM_SLIPPAGE_MULTIPLIER",
        config.execution.clmm_slippage_multiplier,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_EXECUTION__MIN_LIQUIDITY",
        config.execution.min_liquidity,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_EXECUTION__MAX_LOSS_PER_TRADE_USD",
        config.execution.max_loss_per_trade_usd,
        f64
    );

    // ── [scalper] ─────────────────────────────────────────────────────────
    env_scalar!(
        "SOLANA_ARB_SCALPER__TAKE_PROFIT_PCT",
        config.scalper.take_profit_pct,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_SCALPER__STOP_LOSS_PCT",
        config.scalper.stop_loss_pct,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_SCALPER__MIN_EDGE_BPS",
        config.scalper.min_edge_bps,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_SCALPER__TRADE_COOLDOWN_SECS",
        config.scalper.trade_cooldown_secs,
        u64
    );

    // ── [risk] ────────────────────────────────────────────────────────────
    env_scalar!(
        "SOLANA_ARB_RISK__MAX_POSITION_SIZE_USD",
        config.risk.max_position_size_usd,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__MAX_DRAWDOWN_PCT",
        config.risk.max_drawdown_pct,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__DAILY_LOSS_LIMIT_PCT",
        config.risk.daily_loss_limit_pct,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__MONTHLY_LOSS_LIMIT_PCT",
        config.risk.monthly_loss_limit_pct,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__TOTAL_LOSS_HALT_PCT",
        config.risk.total_loss_halt_pct,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__DAILY_PAUSE_SECS",
        config.risk.daily_pause_secs,
        u64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__MIN_LIQUIDITY_USD",
        config.risk.min_liquidity_usd,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__MAX_ROUTE_SLIPPAGE",
        config.risk.max_route_slippage,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__MAX_ROUTE_DEPTH",
        config.risk.max_route_depth,
        usize
    );
    env_scalar!(
        "SOLANA_ARB_RISK__MIN_PRICE_PRODUCT",
        config.risk.min_price_product,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__MAX_PRICE_PRODUCT",
        config.risk.max_price_product,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__DAILY_LOSS_LIMIT_USD",
        config.risk.daily_loss_limit_usd,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_RISK__MAX_OPEN_POSITIONS",
        config.risk.max_open_positions,
        u32
    );
    env_scalar!(
        "SOLANA_ARB_RISK__CAPITAL_DEPLOYED_PCT_MAX",
        config.risk.capital_deployed_pct_max,
        f64
    );

    // ── [features] ────────────────────────────────────────────────────────
    env_bool!(
        "SOLANA_ARB_FEATURES__ENABLE_LIVE_TRADING",
        config.features.enable_live_trading
    );
    env_bool!(
        "SOLANA_ARB_FEATURES__ENABLE_RISK_ENGINE",
        config.features.enable_risk_engine
    );
    env_bool!(
        "SOLANA_ARB_FEATURES__ENABLE_METRICS",
        config.features.enable_metrics
    );
    env_bool!("SOLANA_ARB_FEATURES__DRY_RUN", config.features.dry_run);
    env_bool!("SOLANA_ARB_FEATURES__ENABLE_JITO", config.features.enable_jito);
    env_bool!("SOLANA_ARB_FEATURES__ENABLE_CLMM", config.features.enable_clmm);
    env_bool!(
        "SOLANA_ARB_FEATURES__ENABLE_RAYDIUM",
        config.features.enable_raydium
    );
    env_bool!(
        "SOLANA_ARB_FEATURES__VERBOSE_PIPELINE_LOG",
        config.features.verbose_pipeline_log
    );

    // ── [ingestion] ───────────────────────────────────────────────────────
    env_scalar!(
        "SOLANA_ARB_INGESTION__EVENT_CHANNEL_CAPACITY",
        config.ingestion.event_channel_capacity,
        usize
    );
    env_scalar!(
        "SOLANA_ARB_INGESTION__EVENT_LIMIT",
        config.ingestion.event_limit,
        usize
    );
    env_scalar!(
        "SOLANA_ARB_INGESTION__YIELD_EVERY",
        config.ingestion.yield_every,
        usize
    );

    // ── [pipeline] ────────────────────────────────────────────────────────
    env_scalar!(
        "SOLANA_ARB_PIPELINE__MAX_EVENTS",
        config.pipeline.max_events,
        usize
    );
    env_scalar!(
        "SOLANA_ARB_PIPELINE__EVENT_TIMEOUT_MS",
        config.pipeline.event_timeout_ms,
        u64
    );
    env_scalar!(
        "SOLANA_ARB_PIPELINE__ROUTING_MAX_DEPTH",
        config.pipeline.routing_max_depth,
        usize
    );
    env_scalar!(
        "SOLANA_ARB_PIPELINE__ROUTING_MIN_LIQUIDITY",
        config.pipeline.routing_min_liquidity,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_PIPELINE__ROUTING_DEPTH_PENALTY_BPS",
        config.pipeline.routing_depth_penalty_bps,
        u64
    );

    // ── [portfolio] ───────────────────────────────────────────────────────
    env_scalar!(
        "SOLANA_ARB_PORTFOLIO__CAPITAL_USD",
        config.portfolio.capital_usd,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_PORTFOLIO__BASE_POSITION_PCT",
        config.portfolio.base_position_pct,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_PORTFOLIO__MAX_POSITION_PCT",
        config.portfolio.max_position_pct,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_PORTFOLIO__MIN_POSITION_USD",
        config.portfolio.min_position_usd,
        f64
    );

    // ── [monitoring] ──────────────────────────────────────────────────────
    env_scalar!(
        "SOLANA_ARB_MONITORING__METRICS_PORT",
        config.monitoring.metrics_port,
        u16
    );
    env_bool!(
        "SOLANA_ARB_MONITORING__ENABLE_PROMETHEUS",
        config.monitoring.enable_prometheus
    );
    env_string!("SOLANA_ARB_MONITORING__LOG_LEVEL", config.monitoring.log_level);
    env_bool!("SOLANA_ARB_MONITORING__JSON_LOGS", config.monitoring.json_logs);

    // ── [strategy] ────────────────────────────────────────────────────────
    env_bool!("SOLANA_ARB_STRATEGY__SCALP", config.strategy.scalp);
    env_bool!("SOLANA_ARB_STRATEGY__ARB", config.strategy.arb);
    env_bool!("SOLANA_ARB_STRATEGY__WHALE_COPY", config.strategy.whale_copy);
    env_bool!("SOLANA_ARB_STRATEGY__MOMENTUM", config.strategy.momentum);
    env_bool!("SOLANA_ARB_STRATEGY__SNIPER", config.strategy.sniper);
    env_bool!("SOLANA_ARB_STRATEGY__LIQUIDATION", config.strategy.liquidation);

    // ── [data_sources] ────────────────────────────────────────────────────
    env_string!(
        "SOLANA_ARB_DATA_SOURCES__HELIUS_API_KEY",
        config.data_sources.helius_api_key
    );
    env_string!(
        "SOLANA_ARB_DATA_SOURCES__JUPITER_PRICE",
        config.data_sources.jupiter_price
    );
    env_string!(
        "SOLANA_ARB_DATA_SOURCES__DEXSCREENER_BASE",
        config.data_sources.dexscreener_base
    );
    env_string!(
        "SOLANA_ARB_DATA_SOURCES__RUGCHECK_BASE",
        config.data_sources.rugcheck_base
    );
    env_string!(
        "SOLANA_ARB_DATA_SOURCES__BIRDEYE_BASE",
        config.data_sources.birdeye_base
    );

    // ── [whale_tracker] ───────────────────────────────────────────────────
    env_bool!(
        "SOLANA_ARB_WHALE_TRACKER__ENABLED",
        config.whale_tracker.enabled
    );
    env_scalar!(
        "SOLANA_ARB_WHALE_TRACKER__MIN_TRADE_USD",
        config.whale_tracker.min_trade_usd,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_WHALE_TRACKER__SIGNAL_TTL_SECONDS",
        config.whale_tracker.signal_ttl_seconds,
        u64
    );
    env_scalar!(
        "SOLANA_ARB_WHALE_TRACKER__MAX_TRACKED_WALLETS",
        config.whale_tracker.max_tracked_wallets,
        usize
    );
    env_scalar!(
        "SOLANA_ARB_WHALE_TRACKER__DISCOVERY_INTERVAL_S",
        config.whale_tracker.discovery_interval_s,
        u64
    );
    env_string!(
        "SOLANA_ARB_WHALE_TRACKER__PERSIST_PATH",
        config.whale_tracker.persist_path
    );

    // ── [copy_trading] ────────────────────────────────────────────────────
    env_bool!(
        "SOLANA_ARB_COPY_TRADING__ENABLED",
        config.copy_trading.enabled
    );
    env_scalar!(
        "SOLANA_ARB_COPY_TRADING__COPY_RATIO",
        config.copy_trading.copy_ratio,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_COPY_TRADING__MAX_COPY_USD",
        config.copy_trading.max_copy_usd,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_COPY_TRADING__MIN_WHALE_TRADE_USD",
        config.copy_trading.min_whale_trade_usd,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_COPY_TRADING__MAX_STALENESS_SLOTS",
        config.copy_trading.max_staleness_slots,
        u64
    );
    env_scalar!(
        "SOLANA_ARB_COPY_TRADING__MIN_WALLET_WIN_RATE",
        config.copy_trading.min_wallet_win_rate,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_COPY_TRADING__MIN_WALLET_PNL_30D",
        config.copy_trading.min_wallet_pnl_30d,
        f64
    );
    env_scalar!(
        "SOLANA_ARB_COPY_TRADING__MIN_WALLET_TRADES_30D",
        config.copy_trading.min_wallet_trades_30d,
        u32
    );
    env_scalar!(
        "SOLANA_ARB_COPY_TRADING__MAX_CONCURRENT_COPIES",
        config.copy_trading.max_concurrent_copies,
        u32
    );
    env_bool!(
        "SOLANA_ARB_COPY_TRADING__MIRROR_EXITS",
        config.copy_trading.mirror_exits
    );

    // ── [wallet] ──────────────────────────────────────────────────────────
    env_option_string!(
        "SOLANA_ARB_WALLET__KEYPAIR_ENV_VAR",
        config.wallet.keypair_env_var
    );
    env_string!("SOLANA_ARB_WALLET__RPC_ENDPOINT", config.wallet.rpc_endpoint);
    env_string!("SOLANA_ARB_WALLET__COMMITMENT", config.wallet.commitment);
    env_string!(
        "SOLANA_ARB_WALLET__EXPECTED_NETWORK",
        config.wallet.expected_network
    );
    env_scalar!(
        "SOLANA_ARB_WALLET__MIN_SOL_BALANCE",
        config.wallet.min_sol_balance,
        f64
    );
    env_bool!(
        "SOLANA_ARB_WALLET__VALIDATE_NETWORK_ON_START",
        config.wallet.validate_network_on_start
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn from_toml_str_overrides_scalar_field() {
        let toml = r#"
            [portfolio]
            capital_usd = 5000.0
        "#;

        let handle = ConfigHandle::from_toml_str(toml).expect("load");
        assert_eq!(handle.portfolio.capital_usd, 5000.0);
    }

    #[test]
    fn from_toml_str_fills_missing_sections_with_defaults() {
        let toml = r#"
            [features]
            enable_metrics = true
        "#;

        let handle = ConfigHandle::from_toml_str(toml).expect("load");
        assert!(handle.features.dry_run);
        assert!(!handle.features.enable_live_trading);
        assert!(handle.features.enable_metrics);
        assert_eq!(handle.rpc.timeout_ms, 10_000);
    }

    #[test]
    fn from_toml_str_rejects_invalid_config() {
        let toml = r#"
            [portfolio]
            capital_usd = -1.0
        "#;

        let err = ConfigHandle::from_toml_str(toml).expect_err("should fail");
        assert!(matches!(err, ConfigError::Invalid(_)), "got: {err}");
    }

    #[test]
    fn config_handle_is_arc_backed() {
        let h1 = ConfigHandle::from_config(SystemConfig::default());
        let h2 = h1.clone();
        assert!(Arc::ptr_eq(&h1.arc(), &h2.arc()));
    }

    #[test]
    fn config_handle_deref_gives_system_config() {
        let handle = ConfigHandle::from_config(SystemConfig::default());
        let _cfg: &SystemConfig = &*handle;
        let _ = handle.rpc.primary_endpoint();
        let _ = handle.websocket.ping_interval();
        let _ = handle.capital_usd();
    }

    #[test]
    fn load_from_nonexistent_file_returns_not_found() {
        let err = ConfigHandle::load_from_file("__definitely_does_not_exist__.toml")
            .expect_err("should fail");
        assert!(matches!(err, ConfigError::FileNotFound(_)));
    }

    #[test]
    fn from_toml_str_full_sections_parse_correctly() {
        let toml = r#"
            [rpc]
            endpoints = ["https://mainnet.example.com"]
            timeout_ms = 5000
            max_retries = 2
            commitment = "finalized"
            max_concurrent_requests = 4

            [websocket]
            endpoints = ["wss://mainnet.example.com"]
            reconnect_interval_ms = 500
            ping_interval_ms = 0
            max_reconnect_attempts = 5
            receive_timeout_ms = 20000

            [retry]
            initial_interval_ms = 200
            max_interval_ms = 5000
            multiplier = 1.5
            max_retries = 3
            jitter = false

            [execution]
            max_slippage_bps = 30
            priority_fee_lamports = 1000
            min_profit_threshold_usd = 0.25
            simulation_initial_amount_usd = 25.0
            max_input_ratio = 0.10
            clmm_slippage_multiplier = 0.4
            min_liquidity = 500.0
            max_trade_size_usd = 100.0

            [risk]
            max_position_size_usd = 200.0
            max_drawdown_pct = 0.20
            daily_loss_limit_pct = 0.03
            monthly_loss_limit_pct = 0.10
            total_loss_halt_pct = 0.35
            daily_pause_secs = 7200
            min_liquidity_usd = 1000.0
            max_route_slippage = 0.02
            max_route_depth = 3
            min_price_product = 0.1
            max_price_product = 10.0
            daily_loss_limit_usd = 50.0
            max_open_positions = 10
            capital_deployed_pct_max = 0.80

            [features]
            enable_live_trading = false
            enable_risk_engine = true
            enable_metrics = true
            dry_run = true
            enable_jito = false
            enable_clmm = true
            enable_raydium = true
            verbose_pipeline_log = false

            [ingestion]
            event_channel_capacity = 8192
            event_limit = 0
            yield_every = 512

            [pipeline]
            max_events = 0
            event_timeout_ms = 50
            routing_max_depth = 3
            routing_min_liquidity = 1000.0
            routing_depth_penalty_bps = 30

            [portfolio]
            capital_usd = 10000.0
            base_position_pct = 0.01
            max_position_pct = 0.04
            min_position_usd = 5.0

            [monitoring]
            metrics_port = 9191
            enable_prometheus = true
            log_level = "debug"
            json_logs = true
        "#;

        let handle = ConfigHandle::from_toml_str(toml).expect("full parse");
        handle.validate().expect("validates");

        assert_eq!(handle.rpc.timeout_ms, 5000);
        assert_eq!(handle.portfolio.capital_usd, 10_000.0);
        assert_eq!(handle.capital_usd(), 10_000.0);
        assert!(handle.features.enable_metrics);
        assert!(handle.features.dry_run);
        assert_eq!(handle.monitoring.metrics_port, 9191);
        assert!(handle.monitoring.json_logs);
        assert!(handle.websocket.ping_interval().is_none());
    }

    #[test]
    fn env_overrides_applied_correctly() {
        let mut cfg = SystemConfig::default();
        cfg.portfolio.capital_usd = 99_999.0;
        cfg.monitoring.log_level = "warn".to_owned();

        assert_eq!(cfg.portfolio.capital_usd, 99_999.0);
        assert!(cfg.features.dry_run);
        assert_eq!(cfg.monitoring.log_level, "warn");
        cfg.validate().expect("mutated config is valid");
    }

    #[test]
    fn example_toml_file_parses_correctly() {
        let example_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config.example.toml");
        let content =
            std::fs::read_to_string(example_path).expect("config.example.toml must exist");
        let cfg: SystemConfig = toml::from_str(&content).expect("example TOML must parse");
        cfg.validate().expect("example config must be valid");
    }
}
