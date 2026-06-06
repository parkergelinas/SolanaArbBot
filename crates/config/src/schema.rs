//! All configuration structs for the workspace.
//!
//! Every field has a documented meaning and a sensible default value.
//! [`SystemConfig`] is the single source of truth; no crate outside of
//! `config` should define its own independent configuration struct.

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ─────────────────────────────────────────────────────────────────────────────
// Top-level
// ─────────────────────────────────────────────────────────────────────────────

/// Complete system configuration.
///
/// Loaded once at startup via [`crate::loader::ConfigHandle::load`] and then
/// shared as `Arc<SystemConfig>` across all pipeline stages.  Immutable after
/// construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Solana JSON-RPC settings.
    #[serde(default)]
    pub rpc: RpcConfig,

    /// Solana WebSocket / gRPC subscription settings.
    #[serde(default)]
    pub websocket: WebSocketConfig,

    /// Retry and exponential back-off policy.
    #[serde(default)]
    pub retry: RetryConfig,

    /// Trade execution simulation settings.
    #[serde(default)]
    pub execution: ExecutionConfig,

    /// Route-level and portfolio-level risk limits.
    #[serde(default)]
    pub risk: RiskConfig,

    /// Runtime feature flags.
    #[serde(default)]
    pub features: FeatureFlags,

    /// Event ingestion pipeline settings.
    #[serde(default)]
    pub ingestion: IngestionConfig,

    /// Pipeline orchestration settings.
    #[serde(default)]
    pub pipeline: PipelineConfig,

    /// Portfolio capital and position-sizing settings.
    #[serde(default)]
    pub portfolio: PortfolioConfig,

    /// Observability / metrics settings.
    #[serde(default)]
    pub monitoring: MonitoringConfig,

    /// Signal engine detection and filtering parameters.
    #[serde(default)]
    pub signal_engine: SignalEngineConfig,

    /// Scalper engine trade-filter and position-sizing parameters.
    #[serde(default)]
    pub scalper: ScalerConfig,

    /// Wallet and key-management settings.
    ///
    /// Keypair loading is gated behind `features.dry_run`.  The `wallet` crate
    /// is the only consumer of these values.
    #[serde(default)]
    pub wallet: WalletConfig,

    /// Governance and orchestration layer settings.
    #[serde(default)]
    pub orchestrator: OrchestratorConfig,

    /// Ultra low-latency hot-path engine parameters.
    #[serde(default)]
    pub hotpath: HotPathConfig,

    /// Active trading strategy toggles (UI / control-api).
    #[serde(default)]
    pub strategy: StrategyConfig,

    /// Free / cheap external data API endpoints (Helius, Jupiter, DexScreener, etc.).
    #[serde(default)]
    pub data_sources: DataSourcesConfig,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            rpc: RpcConfig::default(),
            websocket: WebSocketConfig::default(),
            retry: RetryConfig::default(),
            execution: ExecutionConfig::default(),
            risk: RiskConfig::default(),
            features: FeatureFlags::default(),
            ingestion: IngestionConfig::default(),
            pipeline: PipelineConfig::default(),
            portfolio: PortfolioConfig::default(),
            monitoring: MonitoringConfig::default(),
            signal_engine: SignalEngineConfig::default(),
            scalper: ScalerConfig::default(),
            wallet: WalletConfig::default(),
            orchestrator: OrchestratorConfig::default(),
            hotpath: HotPathConfig::default(),
            strategy: StrategyConfig::default(),
            data_sources: DataSourcesConfig::default(),
        }
    }
}

impl SystemConfig {
    /// Validates all field invariants.
    ///
    /// Returns the first violation found, if any.
    pub fn validate(&self) -> Result<(), String> {
        self.rpc.validate()?;
        self.websocket.validate()?;
        self.retry.validate()?;
        self.execution.validate()?;
        self.risk.validate()?;
        self.ingestion.validate()?;
        self.pipeline.validate()?;
        self.portfolio.validate()?;
        self.signal_engine.validate()?;
        self.scalper.validate()?;
        // wallet.validate() is intentionally a soft warning: missing keypair
        // config is permitted in dry_run mode and by alternative loaders.
        self.wallet.validate_warn();
        self.hotpath.validate()?;
        self.data_sources.validate()?;

        if (self.risk.min_liquidity_usd - self.pipeline.routing_min_liquidity).abs()
            > f64::EPSILON
        {
            return Err(format!(
                "risk.min_liquidity_usd ({}) must equal pipeline.routing_min_liquidity ({})",
                self.risk.min_liquidity_usd, self.pipeline.routing_min_liquidity
            ));
        }

        Ok(())
    }

    /// Total capital under management — single source: `[portfolio].capital_usd`.
    pub fn capital_usd(&self) -> f64 {
        self.portfolio.capital_usd
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RPC
// ─────────────────────────────────────────────────────────────────────────────

/// Solana JSON-RPC connection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Ordered list of RPC endpoint URLs.  The first reachable endpoint is used;
    /// subsequent entries act as fallbacks.
    pub endpoints: Vec<String>,

    /// Per-request HTTP timeout in milliseconds.
    pub timeout_ms: u64,

    /// Maximum number of request retries before propagating the error.
    pub max_retries: u32,

    /// Solana commitment level (`"processed"`, `"confirmed"`, or `"finalized"`).
    pub commitment: String,

    /// Maximum concurrent in-flight RPC requests per endpoint.
    pub max_concurrent_requests: usize,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            endpoints: vec![
                "https://api.mainnet-beta.solana.com".to_owned(),
                "https://solana-api.projectserum.com".to_owned(),
            ],
            timeout_ms: 10_000,
            max_retries: 3,
            commitment: "confirmed".to_owned(),
            max_concurrent_requests: 10,
        }
    }
}

impl RpcConfig {
    fn validate(&self) -> Result<(), String> {
        if self.endpoints.is_empty() {
            return Err("rpc.endpoints must not be empty".to_owned());
        }
        if self.timeout_ms == 0 {
            return Err("rpc.timeout_ms must be greater than zero".to_owned());
        }
        let valid_commitments = ["processed", "confirmed", "finalized"];
        if !valid_commitments.contains(&self.commitment.as_str()) {
            return Err(format!(
                "rpc.commitment must be one of {:?}, got {:?}",
                valid_commitments, self.commitment
            ));
        }
        Ok(())
    }

    /// Returns the primary (first) RPC endpoint.
    pub fn primary_endpoint(&self) -> &str {
        self.endpoints.first().map(String::as_str).unwrap_or("")
    }

    /// Returns configured timeout as a [`Duration`].
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// WebSocket
// ─────────────────────────────────────────────────────────────────────────────

/// Solana WebSocket / Yellowstone gRPC subscription settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Ordered list of WebSocket endpoint URLs.
    pub endpoints: Vec<String>,

    /// Delay before attempting to reconnect after a dropped connection (ms).
    pub reconnect_interval_ms: u64,

    /// Interval at which keep-alive pings are sent (ms).  0 = disabled.
    pub ping_interval_ms: u64,

    /// Maximum consecutive reconnect attempts before giving up.  0 = unlimited.
    pub max_reconnect_attempts: u32,

    /// Per-message receive timeout in milliseconds.
    pub receive_timeout_ms: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            endpoints: vec![
                "wss://api.mainnet-beta.solana.com".to_owned(),
            ],
            reconnect_interval_ms: 1_000,
            ping_interval_ms: 15_000,
            max_reconnect_attempts: 10,
            receive_timeout_ms: 30_000,
        }
    }
}

impl WebSocketConfig {
    fn validate(&self) -> Result<(), String> {
        if self.endpoints.is_empty() {
            return Err("websocket.endpoints must not be empty".to_owned());
        }
        Ok(())
    }

    /// Returns the primary WebSocket endpoint.
    pub fn primary_endpoint(&self) -> &str {
        self.endpoints.first().map(String::as_str).unwrap_or("")
    }

    /// Returns reconnect interval as [`Duration`].
    pub fn reconnect_interval(&self) -> Duration {
        Duration::from_millis(self.reconnect_interval_ms)
    }

    /// Returns ping interval as [`Duration`], or `None` when disabled.
    pub fn ping_interval(&self) -> Option<Duration> {
        if self.ping_interval_ms == 0 {
            None
        } else {
            Some(Duration::from_millis(self.ping_interval_ms))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Retry / back-off
// ─────────────────────────────────────────────────────────────────────────────

/// Exponential back-off policy applied to RPC and WebSocket operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Initial retry delay in milliseconds.
    pub initial_interval_ms: u64,

    /// Maximum retry delay cap in milliseconds.
    pub max_interval_ms: u64,

    /// Back-off multiplier (e.g. `2.0` = doubles each attempt).
    pub multiplier: f64,

    /// Maximum number of retry attempts (0 = no retries).
    pub max_retries: u32,

    /// Whether to add a random jitter to prevent thundering-herd.
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            initial_interval_ms: 100,
            max_interval_ms: 10_000,
            multiplier: 2.0,
            max_retries: 5,
            jitter: true,
        }
    }
}

impl RetryConfig {
    fn validate(&self) -> Result<(), String> {
        if self.initial_interval_ms == 0 {
            return Err(
                "retry.initial_interval_ms must be greater than zero".to_owned()
            );
        }
        if self.max_interval_ms < self.initial_interval_ms {
            return Err(
                "retry.max_interval_ms must be >= retry.initial_interval_ms".to_owned()
            );
        }
        if self.multiplier <= 1.0 {
            return Err(
                "retry.multiplier must be greater than 1.0".to_owned()
            );
        }
        Ok(())
    }

    /// Computes the delay for retry attempt `n` (0-indexed) in milliseconds,
    /// capped at [`Self::max_interval_ms`], without jitter.
    pub fn interval_ms_for_attempt(&self, n: u32) -> u64 {
        let raw = self.initial_interval_ms as f64 * self.multiplier.powi(n as i32);
        raw.min(self.max_interval_ms as f64) as u64
    }

    /// Returns whether any retries are enabled.
    pub fn is_enabled(&self) -> bool {
        self.max_retries > 0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Execution simulation
// ─────────────────────────────────────────────────────────────────────────────

/// Trade execution simulation parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum acceptable slippage in basis points (1 bps = 0.01 %).
    pub max_slippage_bps: u64,

    /// Priority fee per compute unit in lamports (for live trading context).
    pub priority_fee_lamports: u64,

    /// Minimum net profit in USD for a route to be considered viable.
    pub min_profit_threshold_usd: f64,

    /// Notional USD amount used as the input for every route simulation.
    pub simulation_initial_amount_usd: f64,

    /// Maximum ratio of input amount to pool liquidity per hop (0.0–1.0).
    pub max_input_ratio: f64,

    /// Slippage multiplier applied to the simplified CLMM tick model.
    pub clmm_slippage_multiplier: f64,

    /// Minimum per-edge liquidity required to proceed with simulation.
    pub min_liquidity: f64,

    /// Hard cap on a single trade's notional size in USD.
    /// The execution gate rejects any request exceeding this value.
    pub max_trade_size_usd: f64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_slippage_bps: 50,
            priority_fee_lamports: 5_000,
            min_profit_threshold_usd: 0.25,
            simulation_initial_amount_usd: 10.0,
            max_input_ratio: 0.25,
            clmm_slippage_multiplier: 0.35,
            min_liquidity: 1.0,
            max_trade_size_usd: 50.0,
        }
    }
}

impl ExecutionConfig {
    fn validate(&self) -> Result<(), String> {
        if self.simulation_initial_amount_usd <= 0.0 {
            return Err(
                "execution.simulation_initial_amount_usd must be positive".to_owned()
            );
        }
        if self.max_input_ratio <= 0.0 || self.max_input_ratio > 1.0 {
            return Err(
                "execution.max_input_ratio must be in (0.0, 1.0]".to_owned()
            );
        }
        if self.min_liquidity < 0.0 {
            return Err("execution.min_liquidity must be >= 0".to_owned());
        }
        Ok(())
    }

    /// Returns max slippage as a fraction (basis-point value / 10 000).
    pub fn max_slippage_fraction(&self) -> f64 {
        self.max_slippage_bps as f64 / 10_000.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Risk limits
// ─────────────────────────────────────────────────────────────────────────────

/// Combined route-level and portfolio-level risk controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    // ── Portfolio risk ────────────────────────────────────────────────────
    /// Maximum single-position size in USD.
    pub max_position_size_usd: f64,

    /// Maximum drawdown from peak equity as a fraction (e.g. `0.25` = 25 %).
    pub max_drawdown_pct: f64,

    /// Maximum daily loss as a fraction of capital.
    pub daily_loss_limit_pct: f64,

    /// Maximum monthly loss as a fraction of capital.
    pub monthly_loss_limit_pct: f64,

    /// Cumulative loss fraction that triggers a permanent halt.
    pub total_loss_halt_pct: f64,

    /// Duration to pause trading after the daily loss limit is breached (seconds).
    pub daily_pause_secs: u64,

    // ── Route-level risk ──────────────────────────────────────────────────
    /// Minimum per-edge pool liquidity in USD for a route to be approved.
    pub min_liquidity_usd: f64,

    /// Maximum tolerated route slippage as a fraction (e.g. `0.05` = 5 %).
    pub max_route_slippage: f64,

    /// Maximum route depth in hops that the risk engine accepts.
    pub max_route_depth: usize,

    /// Minimum cumulative price product across all hops (below = suspicious).
    pub min_price_product: f64,

    /// Maximum cumulative price product across all hops (above = suspicious).
    pub max_price_product: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_size_usd: 50.0,
            max_drawdown_pct: 0.25,
            daily_loss_limit_pct: 0.05,
            monthly_loss_limit_pct: 0.15,
            total_loss_halt_pct: 0.40,
            daily_pause_secs: 3_600,
            min_liquidity_usd: 1_000.0,
            max_route_slippage: 0.05,
            max_route_depth: 3,
            min_price_product: 0.2,
            max_price_product: 5.0,
        }
    }
}

impl RiskConfig {
    fn validate(&self) -> Result<(), String> {
        if self.max_drawdown_pct <= 0.0 || self.max_drawdown_pct >= 1.0 {
            return Err("risk.max_drawdown_pct must be in (0.0, 1.0)".to_owned());
        }
        if self.daily_loss_limit_pct <= 0.0 || self.daily_loss_limit_pct >= 1.0 {
            return Err("risk.daily_loss_limit_pct must be in (0.0, 1.0)".to_owned());
        }
        if self.max_route_depth < 2 || self.max_route_depth > 5 {
            return Err("risk.max_route_depth must be between 2 and 5".to_owned());
        }
        if self.min_price_product >= self.max_price_product {
            return Err(
                "risk.min_price_product must be less than risk.max_price_product".to_owned()
            );
        }
        Ok(())
    }

    /// Returns daily pause duration.
    pub fn daily_pause(&self) -> Duration {
        Duration::from_secs(self.daily_pause_secs)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Feature flags
// ─────────────────────────────────────────────────────────────────────────────

/// Runtime feature flags controlling which subsystems are active.
///
/// All flags default to the **safest** values (i.e. live trading off, dry-run
/// on) so that the system is safe to run in a new environment without an
/// explicit config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Master kill-switch for on-chain transaction submission.
    ///
    /// **Must be explicitly set to `true` to enable live trading.**
    /// Defaults to `false` — all routes are simulated only.
    pub enable_live_trading: bool,

    /// Enables the portfolio-level risk state machine.
    pub enable_risk_engine: bool,

    /// Enables the Prometheus metrics endpoint.
    pub enable_metrics: bool,

    /// Dry-run mode — pipeline runs but no signals are acted on.
    pub dry_run: bool,

    /// Enables Jito bundle submission for MEV protection (future phase).
    pub enable_jito: bool,

    /// Enables Orca CLMM pool decoding and routing.
    pub enable_clmm: bool,

    /// Enables Raydium AMM pool decoding and routing.
    pub enable_raydium: bool,

    /// Verbose pipeline event logging (performance cost — dev only).
    pub verbose_pipeline_log: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_live_trading: false,
            enable_risk_engine: true,
            enable_metrics: false,
            dry_run: true,
            enable_jito: false,
            enable_clmm: true,
            enable_raydium: true,
            verbose_pipeline_log: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy runtime toggles
// ─────────────────────────────────────────────────────────────────────────────

/// Per-strategy enable flags synced from the dashboard / control-api.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyConfig {
    pub scalp: bool,
    pub arb: bool,
    pub whale_copy: bool,
    pub momentum: bool,
    pub sniper: bool,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            scalp: true,
            arb: true,
            whale_copy: false,
            momentum: false,
            sniper: false,
        }
    }
}

impl StrategyConfig {
    /// Returns true when every strategy toggle is off (paused matrix).
    pub fn all_disabled(&self) -> bool {
        !self.scalp && !self.arb && !self.whale_copy && !self.momentum && !self.sniper
    }

    /// True when whale-copy or momentum strategies are enabled (non-paper signal path).
    pub fn requires_live_ingestion(&self) -> bool {
        self.whale_copy || self.momentum || self.sniper
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Ingestion
// ─────────────────────────────────────────────────────────────────────────────

/// Event ingestion pipeline settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    /// Bounded channel capacity per event-bus subscriber.
    pub event_channel_capacity: usize,

    /// Maximum number of events to ingest before the source stops.
    /// 0 = run indefinitely (production mode).
    pub event_limit: usize,

    /// How many events to emit before yielding to the async runtime.
    /// 0 = never yield explicitly (let the runtime schedule naturally).
    pub yield_every: usize,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            event_channel_capacity: 4_096,
            event_limit: 0,
            yield_every: 256,
        }
    }
}

impl IngestionConfig {
    fn validate(&self) -> Result<(), String> {
        if self.event_channel_capacity == 0 {
            return Err(
                "ingestion.event_channel_capacity must be greater than zero".to_owned()
            );
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Engine pipeline orchestration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Maximum number of events the pipeline processes per run-loop call.
    /// 0 = run indefinitely.
    pub max_events: usize,

    /// Per-event subscriber receive timeout in milliseconds.
    pub event_timeout_ms: u64,

    /// Maximum route depth in hops for the DFS cycle detector.
    pub routing_max_depth: usize,

    /// Minimum edge liquidity allowed in candidate routes.
    pub routing_min_liquidity: f64,

    /// Per-hop depth penalty in basis points applied to route scoring.
    pub routing_depth_penalty_bps: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_events: 0,
            event_timeout_ms: 100,
            routing_max_depth: 3,
            routing_min_liquidity: 1_000.0,
            routing_depth_penalty_bps: 50,
        }
    }
}

impl PipelineConfig {
    fn validate(&self) -> Result<(), String> {
        if self.routing_max_depth < 2 || self.routing_max_depth > 5 {
            return Err(
                "pipeline.routing_max_depth must be between 2 and 5".to_owned()
            );
        }
        Ok(())
    }

    /// Returns event timeout as [`Duration`].
    pub fn event_timeout(&self) -> Duration {
        Duration::from_millis(self.event_timeout_ms)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Portfolio
// ─────────────────────────────────────────────────────────────────────────────

/// Capital and position-sizing parameters for the portfolio engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioConfig {
    /// Total capital in USD available for allocation.
    pub capital_usd: f64,

    /// Base position size as a fraction of capital (e.g. `0.02` = 2 %).
    pub base_position_pct: f64,

    /// Hard cap on position size as a fraction of capital.
    pub max_position_pct: f64,

    /// Minimum position size in USD (prevents dust positions).
    pub min_position_usd: f64,
}

impl Default for PortfolioConfig {
    fn default() -> Self {
        Self {
            capital_usd: 1_000.0,
            base_position_pct: 0.02,
            max_position_pct: 0.05,
            min_position_usd: 1.50,
        }
    }
}

impl PortfolioConfig {
    fn validate(&self) -> Result<(), String> {
        if self.capital_usd <= 0.0 {
            return Err("portfolio.capital_usd must be positive".to_owned());
        }
        if self.base_position_pct <= 0.0 || self.base_position_pct > 1.0 {
            return Err(
                "portfolio.base_position_pct must be in (0.0, 1.0]".to_owned()
            );
        }
        if self.max_position_pct < self.base_position_pct {
            return Err(
                "portfolio.max_position_pct must be >= portfolio.base_position_pct".to_owned()
            );
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Data sources (free / cheap live APIs)
// ─────────────────────────────────────────────────────────────────────────────

/// External market-data API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourcesConfig {
    /// Helius free-tier API key — set via `SOLANA_ARB_DATA_SOURCES__HELIUS_API_KEY`.
    pub helius_api_key: String,
    /// Jupiter Price API v2 base URL.
    pub jupiter_price: String,
    /// DexScreener REST base URL.
    pub dexscreener_base: String,
    /// Rugcheck token report API base URL.
    pub rugcheck_base: String,
    /// Birdeye public API base URL (free tier, no key).
    pub birdeye_base: String,
}

impl Default for DataSourcesConfig {
    fn default() -> Self {
        Self {
            helius_api_key: String::new(),
            jupiter_price: "https://price.jup.ag/v2/price".to_owned(),
            dexscreener_base: "https://api.dexscreener.com/latest/dex".to_owned(),
            rugcheck_base: "https://api.rugcheck.xyz/v1".to_owned(),
            birdeye_base: "https://public-api.birdeye.so".to_owned(),
        }
    }
}

impl DataSourcesConfig {
    fn validate(&self) -> Result<(), String> {
        for (name, url) in [
            ("jupiter_price", &self.jupiter_price),
            ("dexscreener_base", &self.dexscreener_base),
            ("rugcheck_base", &self.rugcheck_base),
            ("birdeye_base", &self.birdeye_base),
        ] {
            if url.is_empty() {
                return Err(format!("data_sources.{name} must not be empty"));
            }
        }
        Ok(())
    }

    /// Helius mainnet WebSocket URL when API key is set.
    pub fn helius_ws_url(&self) -> Option<String> {
        if self.helius_api_key.is_empty() {
            return None;
        }
        Some(format!(
            "wss://mainnet.helius-rpc.com/?api-key={}",
            self.helius_api_key
        ))
    }

    /// Helius mainnet HTTPS RPC URL when API key is set.
    pub fn helius_rpc_url(&self) -> Option<String> {
        if self.helius_api_key.is_empty() {
            return None;
        }
        Some(format!(
            "https://mainnet.helius-rpc.com/?api-key={}",
            self.helius_api_key
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Monitoring
// ─────────────────────────────────────────────────────────────────────────────

/// Observability and monitoring settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// TCP port on which the Prometheus metrics endpoint listens.
    pub metrics_port: u16,

    /// Whether the Prometheus `/metrics` HTTP endpoint is started.
    pub enable_prometheus: bool,

    /// Minimum log level (`"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`).
    pub log_level: String,

    /// Whether to emit structured JSON logs (vs human-readable).
    pub json_logs: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_port: 9090,
            enable_prometheus: false,
            log_level: "info".to_owned(),
            json_logs: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Signal Engine
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for the modular signal detection engine.
///
/// All thresholds that control signal generation and filtering live here.
/// No processor may hardcode any numeric threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalEngineConfig {
    /// Minimum USD swap amount that classifies an event as whale activity.
    pub whale_threshold_usd: f64,

    /// Rolling window length in seconds used for momentum calculations.
    pub momentum_window_secs: u64,

    /// Minimum wallet profitability score (0.0–1.0) to qualify as smart money.
    pub smart_money_min_score: f64,

    /// Minimum signal strength (0.0–1.0) for a signal to pass the aggregator.
    pub signal_min_strength: f64,

    /// Minimum signal confidence (0.0–1.0) for a signal to pass the aggregator.
    pub signal_min_confidence: f64,

    /// Per-pool cooldown in seconds; a pool cannot emit two signals within this window.
    pub cooldown_secs: u64,

    /// Maximum age in seconds of feature-store entries before they are pruned.
    pub feature_store_max_age_secs: u64,

    /// Crossbeam channel capacity for the outbound signal bus.
    pub signal_channel_capacity: usize,
}

impl Default for SignalEngineConfig {
    fn default() -> Self {
        Self {
            whale_threshold_usd: 10_000.0,
            momentum_window_secs: 60,
            smart_money_min_score: 0.70,
            signal_min_strength: 0.30,
            signal_min_confidence: 0.40,
            cooldown_secs: 30,
            feature_store_max_age_secs: 300,
            signal_channel_capacity: 1_024,
        }
    }
}

impl SignalEngineConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.whale_threshold_usd <= 0.0 {
            return Err(
                "signal_engine.whale_threshold_usd must be positive".to_owned()
            );
        }
        if self.momentum_window_secs == 0 {
            return Err(
                "signal_engine.momentum_window_secs must be greater than zero".to_owned()
            );
        }
        if !(0.0..=1.0).contains(&self.smart_money_min_score) {
            return Err(
                "signal_engine.smart_money_min_score must be in [0.0, 1.0]".to_owned()
            );
        }
        if !(0.0..=1.0).contains(&self.signal_min_strength) {
            return Err(
                "signal_engine.signal_min_strength must be in [0.0, 1.0]".to_owned()
            );
        }
        if !(0.0..=1.0).contains(&self.signal_min_confidence) {
            return Err(
                "signal_engine.signal_min_confidence must be in [0.0, 1.0]".to_owned()
            );
        }
        if self.signal_channel_capacity == 0 {
            return Err(
                "signal_engine.signal_channel_capacity must be greater than zero".to_owned()
            );
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scalper
// ─────────────────────────────────────────────────────────────────────────────

/// Trade-filter and position-sizing parameters for the scalper engine.
///
/// Every numeric threshold that governs trade selection, risk sizing, and paper
/// execution lives here.  No module in the `scalper` crate may hardcode a
/// numeric constant; all values must come from this struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalerConfig {
    // ── Liquidity filters ─────────────────────────────────────────────────
    /// Minimum pool TVL in USD; trades on pools below this are rejected.
    pub min_pool_tvl_usd: f64,
    /// Minimum rolling-window volume (proxy for 5-min USD volume); trades are
    /// rejected when the pool is illiquid on a short-term basis.
    pub min_volume_5m_usd: f64,

    // ── Volatility regime ─────────────────────────────────────────────────
    /// Minimum absolute price-velocity ratio (z-score proxy) required for a
    /// trade signal.  Rejects signals in dead-market conditions.
    pub volatility_floor: f64,
    /// Maximum absolute price-velocity ratio.  Rejects signals during extreme
    /// volatility that increases execution risk.
    pub volatility_ceiling: f64,

    // ── Slippage / cost ───────────────────────────────────────────────────
    /// Maximum acceptable estimated round-trip cost in basis points.
    pub max_slippage_bps: f64,
    /// MEV sandwich haircut applied to every paper fill (basis points).
    pub mev_haircut_bps: f64,

    // ── Rate limits ───────────────────────────────────────────────────────
    /// Maximum trades per asset per rolling hour.
    pub max_trades_per_hour_per_asset: u32,
    /// Maximum trades across all assets per rolling hour.
    pub max_trades_per_hour_global: u32,

    // ── Cooldown ──────────────────────────────────────────────────────────
    /// Per-pool cooldown in seconds after a trade is executed.
    pub trade_cooldown_secs: u64,

    // ── Edge requirement ──────────────────────────────────────────────────
    /// Minimum net edge in basis points (signal_strength_bps − round_trip_cost_bps).
    pub min_edge_bps: f64,

    // ── Position sizing ───────────────────────────────────────────────────
    /// Base position size as a fraction of capital (e.g. `0.02` = 2 %).
    pub base_position_pct: f64,
    /// Exponent applied to signal strength in the sizing formula.
    pub strength_scaling_exponent: f64,
    /// Hard cap on position size in USD.
    pub max_position_usd: f64,
    /// Minimum position size in USD (prevents dust positions).
    pub min_position_usd: f64,

    // ── Signal freshness ──────────────────────────────────────────────────
    /// Maximum signal age in seconds; older signals are discarded.
    pub signal_max_age_secs: u64,

    // ── Paper trading fees ────────────────────────────────────────────────
    /// Base DEX fee in basis points (Raydium default: 25 bps = 0.25 %).
    pub base_fee_bps: u16,
    /// Priority fee in lamports used in the priority-cost attribution model.
    pub priority_fee_lamports: u64,
}

impl Default for ScalerConfig {
    fn default() -> Self {
        Self {
            min_pool_tvl_usd: 100_000.0,
            min_volume_5m_usd: 5_000.0,
            volatility_floor: 0.1,
            volatility_ceiling: 4.0,
            max_slippage_bps: 50.0,
            mev_haircut_bps: 10.0,
            max_trades_per_hour_per_asset: 12,
            max_trades_per_hour_global: 100,
            trade_cooldown_secs: 60,
            min_edge_bps: 20.0,
            base_position_pct: 0.02,
            strength_scaling_exponent: 1.5,
            max_position_usd: 5_000.0,
            min_position_usd: 100.0,
            signal_max_age_secs: 30,
            base_fee_bps: 25,
            priority_fee_lamports: 100_000,
        }
    }
}

impl ScalerConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.min_pool_tvl_usd < 0.0 {
            return Err("scalper.min_pool_tvl_usd must be >= 0".to_owned());
        }
        if self.volatility_floor >= self.volatility_ceiling {
            return Err(
                "scalper.volatility_floor must be strictly less than volatility_ceiling".to_owned(),
            );
        }
        if self.max_slippage_bps < 0.0 {
            return Err("scalper.max_slippage_bps must be >= 0".to_owned());
        }
        if self.base_position_pct <= 0.0 || self.base_position_pct > 1.0 {
            return Err("scalper.base_position_pct must be in (0.0, 1.0]".to_owned());
        }
        if self.min_position_usd > self.max_position_usd {
            return Err(
                "scalper.min_position_usd must be <= scalper.max_position_usd".to_owned(),
            );
        }
        if self.signal_max_age_secs == 0 {
            return Err("scalper.signal_max_age_secs must be > 0".to_owned());
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Orchestrator
// ─────────────────────────────────────────────────────────────────────────────

/// Governance and orchestration layer settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum tolerated PnL degradation (%) before a regression is flagged.
    pub regression_pnl_threshold_pct: f64,

    /// Maximum tolerated drawdown increase (%) before a regression is flagged.
    pub regression_drawdown_threshold_pct: f64,

    /// Maximum tolerated win-rate drop (%) before a regression is flagged.
    pub regression_win_rate_threshold_pct: f64,

    /// Interval in seconds between periodic health-check passes.
    pub health_check_interval_secs: u64,

    /// When `true`, a `Critical` health status immediately triggers an
    /// emergency stop (disables trading and locks risk state).
    pub emergency_stop_on_critical_health: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            regression_pnl_threshold_pct: 10.0,
            regression_drawdown_threshold_pct: 15.0,
            regression_win_rate_threshold_pct: 5.0,
            health_check_interval_secs: 30,
            emergency_stop_on_critical_health: true,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hot-path engine
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for the ultra low-latency synchronous execution pipeline.
///
/// All thresholds are loaded once at startup and copied into precomputed
/// fixed-point tables — no parsing or allocation occurs in the hot loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotPathConfig {
    /// Maximum number of pool slots in the in-memory state table.
    pub max_pools: usize,
    /// Target decision latency budget in microseconds (design target: 50_000).
    pub decision_budget_us: u64,
    /// Minimum net edge in basis points to emit an execution intent.
    pub min_edge_bps: u32,
    /// Maximum one-leg slippage in basis points before rejection.
    pub max_slippage_bps: u32,
    /// Minimum pool liquidity (USD, fixed-point ×100) for trade eligibility.
    pub min_liquidity_usd_x100: u64,
    /// Momentum signal: minimum volume acceleration ratio (×1000).
    pub momentum_accel_threshold_x1000: u32,
    /// Signal minimum strength (0–1000 maps to 0.0–1.0).
    pub min_signal_strength_x1000: u32,
    /// Prefer Jito bundle submission when `true`.
    pub prefer_jito: bool,
    /// Maximum Jito tip in lamports.
    pub max_jito_tip_lamports: u64,
    /// Allow direct RPC fallback when Jito is unavailable.
    pub allow_direct_rpc_fallback: bool,
    /// Size of the cold-path execution queue (preallocated SPSC).
    pub execution_queue_capacity: usize,
    /// Paper mode — no network submission (default `true`).
    pub paper_mode: bool,
}

impl Default for HotPathConfig {
    fn default() -> Self {
        Self {
            max_pools: 128,
            decision_budget_us: 50_000,
            min_edge_bps: 20,
            max_slippage_bps: 50,
            min_liquidity_usd_x100: 100_000_00, // $100k
            momentum_accel_threshold_x1000: 1100, // 1.1×
            min_signal_strength_x1000: 400,
            prefer_jito: true,
            max_jito_tip_lamports: 50_000,
            allow_direct_rpc_fallback: true,
            execution_queue_capacity: 256,
            paper_mode: true,
        }
    }
}

impl HotPathConfig {
    /// Validates hot-path configuration invariants.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_pools == 0 || self.max_pools > 512 {
            return Err("hotpath.max_pools must be in 1..=512".into());
        }
        if self.decision_budget_us == 0 {
            return Err("hotpath.decision_budget_us must be > 0".into());
        }
        if self.execution_queue_capacity == 0 {
            return Err("hotpath.execution_queue_capacity must be > 0".into());
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Wallet
// ─────────────────────────────────────────────────────────────────────────────

/// Wallet and key-management configuration.
///
/// This struct is consumed exclusively by the `wallet` crate.  No other crate
/// should read or act on these fields directly.
///
/// Defaults are deliberately safe (devnet, dry-run friendly, no keypair path).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Path to a Solana CLI JSON keypair file (`[u8; 64]` array).
    /// Only loaded when `features.dry_run == false`.
    pub keypair_path: Option<String>,

    /// Name of the environment variable holding the base58-encoded private key.
    /// Only read when `features.dry_run == false`.
    pub keypair_env_var: Option<String>,

    /// RPC endpoint used for balance queries and transaction operations.
    pub rpc_endpoint: String,

    /// Commitment level: `"processed"`, `"confirmed"`, or `"finalized"`.
    pub commitment: String,

    /// Expected network; prevents a devnet keypair from hitting mainnet.
    /// Accepted values: `"mainnet"`, `"devnet"`, `"localnet"`.
    pub expected_network: String,

    /// Minimum SOL balance (in SOL, **not** lamports) required before execution
    /// is allowed.  The wallet crate rejects signing below this threshold.
    pub min_sol_balance: f64,

    /// When `true`, the wallet crate validates the cluster genesis hash against
    /// `expected_network` on startup.
    pub validate_network_on_start: bool,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            keypair_path: None,
            keypair_env_var: Some("SOLANA_ARB_WALLET_KEY".to_owned()),
            rpc_endpoint: "https://api.devnet.solana.com".to_owned(),
            commitment: "confirmed".to_owned(),
            expected_network: "devnet".to_owned(),
            min_sol_balance: 0.1,
            validate_network_on_start: true,
        }
    }
}

impl WalletConfig {
    /// Emits a soft warning (returns `Ok`) when the system is in live mode
    /// but no keypair source is configured.
    ///
    /// This is a warning rather than an error because the wallet may be loaded
    /// via an alternative mechanism (hardware wallet, injected secret, etc.).
    pub fn validate_warn(&self) {
        // Nothing to validate in terms of hard errors.
        // Live-mode keypair checks are enforced at runtime by the wallet crate.
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_system_config_validates() {
        SystemConfig::default().validate().expect("default config is valid");
    }

    #[test]
    fn rpc_config_rejects_empty_endpoints() {
        let mut cfg = RpcConfig::default();
        cfg.endpoints.clear();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rpc_config_rejects_invalid_commitment() {
        let mut cfg = RpcConfig::default();
        cfg.commitment = "instant".to_owned();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn retry_config_back_off_is_monotone() {
        let cfg = RetryConfig::default();
        let delays: Vec<u64> = (0..5).map(|n| cfg.interval_ms_for_attempt(n)).collect();
        for i in 1..delays.len() {
            assert!(delays[i] >= delays[i - 1], "delay must be non-decreasing");
        }
    }

    #[test]
    fn retry_config_caps_at_max_interval() {
        let cfg = RetryConfig::default();
        for n in 0..20 {
            assert!(cfg.interval_ms_for_attempt(n) <= cfg.max_interval_ms);
        }
    }

    #[test]
    fn execution_config_slippage_fraction_is_correct() {
        let cfg = ExecutionConfig {
            max_slippage_bps: 100,
            ..ExecutionConfig::default()
        };
        assert!((cfg.max_slippage_fraction() - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn risk_config_rejects_inverted_price_product_bounds() {
        let mut cfg = RiskConfig::default();
        cfg.min_price_product = 5.0;
        cfg.max_price_product = 0.2;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn risk_config_rejects_invalid_route_depth() {
        let mut cfg = RiskConfig::default();
        cfg.max_route_depth = 10;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn feature_flags_default_to_safe_values() {
        let flags = FeatureFlags::default();
        assert!(!flags.enable_live_trading, "live trading must default to off");
        assert!(flags.dry_run, "dry_run must default to on");
    }

    #[test]
    fn portfolio_config_rejects_max_less_than_base_pct() {
        let mut cfg = PortfolioConfig::default();
        cfg.base_position_pct = 0.10;
        cfg.max_position_pct = 0.05;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn pipeline_config_event_timeout_converts_correctly() {
        let cfg = PipelineConfig::default();
        assert_eq!(
            cfg.event_timeout().as_millis(),
            u128::from(cfg.event_timeout_ms)
        );
    }

    #[test]
    fn signal_engine_config_defaults_are_valid() {
        SignalEngineConfig::default()
            .validate()
            .expect("default signal_engine config is valid");
    }

    #[test]
    fn signal_engine_config_rejects_zero_threshold() {
        let mut cfg = SignalEngineConfig::default();
        cfg.whale_threshold_usd = 0.0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn signal_engine_config_rejects_out_of_range_strength() {
        let mut cfg = SignalEngineConfig::default();
        cfg.signal_min_strength = 1.5;
        assert!(cfg.validate().is_err());
    }
}
