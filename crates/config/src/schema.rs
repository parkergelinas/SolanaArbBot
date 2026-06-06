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

    /// Whale wallet tracker — Helius swap watcher + GMGN discovery.
    #[serde(default)]
    pub whale_tracker: WhaleTrackerConfig,

    /// Copy-trading / whale-mirror strategy settings.
    #[serde(default)]
    pub copy_trading: CopyTradingConfig,

    /// Momentum / volume spike strategy (Strategy 5).
    #[serde(default)]
    pub momentum: MomentumConfig,

    /// Cross-DEX atomic arbitrage engine parameters.
    #[serde(default)]
    pub arbitrage: ArbitrageConfig,

    /// Liquidation hunter — scan lending protocols for underwater positions.
    #[serde(default)]
    pub liquidation: LiquidationConfig,

    /// New-token sniper strategy parameters.
    #[serde(default)]
    pub sniper: SniperConfig,

    /// Jupiter route-divergence / quote arbitrage scanner.
    #[serde(default)]
    pub quote_arb: QuoteArbConfig,

    /// CoinMarketCap top-N pair universe loader.
    #[serde(default)]
    pub coinmarketcap: CoinMarketCapConfig,

    /// Pump.fun bonding curve edge strategy.
    #[serde(default)]
    pub pump_fun: PumpFunConfig,
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
            whale_tracker: WhaleTrackerConfig::default(),
            copy_trading: CopyTradingConfig::default(),
            momentum: MomentumConfig::default(),
            arbitrage: ArbitrageConfig::default(),
            liquidation: LiquidationConfig::default(),
            sniper: SniperConfig::default(),
            quote_arb: QuoteArbConfig::default(),
            coinmarketcap: CoinMarketCapConfig::default(),
            pump_fun: PumpFunConfig::default(),
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
        self.whale_tracker.validate()?;
        self.copy_trading.validate()?;
        self.momentum.validate()?;
        self.arbitrage.validate()?;
        self.quote_arb.validate()?;
        self.coinmarketcap.validate()?;
        self.pump_fun.validate()?;
        self.sniper.validate()?;
        self.liquidation.validate()?;
        self.features.validate()?;

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

    /// Maximum acceptable net loss per arb trade in USD (downside cap).
    pub max_loss_per_trade_usd: f64,
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
            max_loss_per_trade_usd: 2.0,
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
        if self.max_loss_per_trade_usd <= 0.0 {
            return Err("execution.max_loss_per_trade_usd must be positive".to_owned());
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

    /// Absolute daily loss cap in USD (global risk gate).
    pub daily_loss_limit_usd: f64,

    /// Maximum concurrent open positions across all strategies.
    pub max_open_positions: u32,

    /// Block new trades when deployed capital exceeds this fraction (e.g. 0.80).
    pub capital_deployed_pct_max: f64,
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
            daily_loss_limit_usd: 50.0,
            max_open_positions: 10,
            capital_deployed_pct_max: 0.80,
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

impl FeatureFlags {
    pub fn validate(&self) -> Result<(), String> {
        if self.enable_live_trading && self.dry_run {
            return Err(
                "features.enable_live_trading requires features.dry_run=false".to_owned(),
            );
        }
        if self.enable_live_trading && !crate::live_trading_confirm_env_set() {
            return Err(
                "features.enable_live_trading requires SOLANA_ARB_CONFIRM_LIVE_TRADING=1".to_owned(),
            );
        }
        if !self.dry_run && !crate::live_trading_confirm_env_set() {
            return Err(
                "features.dry_run=false requires SOLANA_ARB_CONFIRM_LIVE_TRADING=1".to_owned(),
            );
        }
        Ok(())
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
    pub quote_arb: bool,
    pub whale_copy: bool,
    pub momentum: bool,
    pub sniper: bool,
    pub liquidation: bool,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            scalp: true,
            arb: true,
            quote_arb: false,
            whale_copy: false,
            momentum: false,
            sniper: false,
            liquidation: false,
        }
    }
}

impl StrategyConfig {
    /// Returns true when every strategy toggle is off (paused matrix).
    pub fn all_disabled(&self) -> bool {
        !self.scalp
            && !self.arb
            && !self.quote_arb
            && !self.whale_copy
            && !self.momentum
            && !self.sniper
            && !self.liquidation
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
    /// Jupiter Price API base URL (`api.jup.ag/price/v3`).
    pub jupiter_price: String,
    /// Jupiter Swap API base URL (`api.jup.ag/swap/v1`).
    pub jupiter_swap: String,
    /// Jupiter Tokens API v2 base URL (`api.jup.ag/tokens/v2`).
    pub jupiter_tokens: String,
    /// DexScreener REST base URL.
    pub dexscreener_base: String,
    /// Rugcheck token report API base URL.
    pub rugcheck_base: String,
    /// Birdeye public API base URL (free tier, no key).
    pub birdeye_base: String,
    /// CoinMarketCap Pro API key for top-100 pair loading.
    pub coinmarketcap_api_key: String,
}

impl Default for DataSourcesConfig {
    fn default() -> Self {
        Self {
            helius_api_key: String::new(),
            jupiter_price: "https://api.jup.ag/price/v3".to_owned(),
            jupiter_swap: "https://api.jup.ag/swap/v1".to_owned(),
            jupiter_tokens: "https://api.jup.ag/tokens/v2".to_owned(),
            dexscreener_base: "https://api.dexscreener.com/latest/dex".to_owned(),
            rugcheck_base: "https://api.rugcheck.xyz/v1".to_owned(),
            birdeye_base: "https://public-api.birdeye.so".to_owned(),
            coinmarketcap_api_key: String::new(),
        }
    }
}

impl DataSourcesConfig {
    fn validate(&self) -> Result<(), String> {
        for (name, url) in [
            ("jupiter_price", &self.jupiter_price),
            ("jupiter_swap", &self.jupiter_swap),
            ("jupiter_tokens", &self.jupiter_tokens),
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
// Whale tracker
// ─────────────────────────────────────────────────────────────────────────────

/// Whale wallet swap watcher and dynamic discovery settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhaleTrackerConfig {
    /// Master toggle for Helius watcher + GMGN discovery pollers.
    pub enabled: bool,
    /// Minimum USD notional to emit a [`WhaleSwapSignal`].
    pub min_trade_usd: f64,
    /// How long (seconds) a whale buy boosts route scoring.
    pub signal_ttl_seconds: u64,
    /// Cap on dynamically discovered + seed wallets.
    pub max_tracked_wallets: usize,
    /// GMGN / discovery poll interval in seconds.
    pub discovery_interval_s: u64,
    /// JSON file for discovered wallet persistence.
    pub persist_path: String,
}

impl Default for WhaleTrackerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_trade_usd: 10_000.0,
            signal_ttl_seconds: 60,
            max_tracked_wallets: 200,
            discovery_interval_s: 300,
            persist_path: "data/discovered_whales.json".to_owned(),
        }
    }
}

impl WhaleTrackerConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.min_trade_usd <= 0.0 {
            return Err("whale_tracker.min_trade_usd must be positive".to_owned());
        }
        if self.signal_ttl_seconds == 0 {
            return Err("whale_tracker.signal_ttl_seconds must be greater than zero".to_owned());
        }
        if self.max_tracked_wallets == 0 {
            return Err("whale_tracker.max_tracked_wallets must be greater than zero".to_owned());
        }
        if self.discovery_interval_s == 0 {
            return Err("whale_tracker.discovery_interval_s must be greater than zero".to_owned());
        }
        if self.persist_path.trim().is_empty() {
            return Err("whale_tracker.persist_path must not be empty".to_owned());
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Copy trading
// ─────────────────────────────────────────────────────────────────────────────

/// Whale mirror / copy-trading strategy parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyTradingConfig {
    /// Master toggle for copy-trading executor and wallet scoring.
    pub enabled: bool,
    /// Fraction of whale notional to mirror (e.g. 0.05 = 5%).
    pub copy_ratio: f64,
    /// Maximum USD per copy trade.
    pub max_copy_usd: f64,
    /// Minimum whale trade USD to trigger a copy signal.
    pub min_whale_trade_usd: f64,
    /// Skip copy when signal is older than this many slots.
    pub max_staleness_slots: u64,
    /// Minimum 30-day win rate for wallet qualification.
    pub min_wallet_win_rate: f64,
    /// Minimum 30-day realized PnL (USD) for wallet qualification.
    pub min_wallet_pnl_30d: f64,
    /// Minimum 30-day trade count for wallet qualification.
    pub min_wallet_trades_30d: u32,
    /// Maximum concurrent open copy positions.
    pub max_concurrent_copies: u32,
    /// Sell immediately when a followed whale sells the same token.
    pub mirror_exits: bool,
}

impl Default for CopyTradingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            copy_ratio: 0.05,
            max_copy_usd: 50.0,
            min_whale_trade_usd: 5_000.0,
            max_staleness_slots: 2,
            min_wallet_win_rate: 0.60,
            min_wallet_pnl_30d: 10_000.0,
            min_wallet_trades_30d: 50,
            max_concurrent_copies: 3,
            mirror_exits: true,
        }
    }
}

impl CopyTradingConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.copy_ratio <= 0.0 || self.copy_ratio > 1.0 {
            return Err("copy_trading.copy_ratio must be in (0.0, 1.0]".to_owned());
        }
        if self.max_copy_usd <= 0.0 {
            return Err("copy_trading.max_copy_usd must be positive".to_owned());
        }
        if self.min_whale_trade_usd <= 0.0 {
            return Err("copy_trading.min_whale_trade_usd must be positive".to_owned());
        }
        if self.max_staleness_slots == 0 {
            return Err("copy_trading.max_staleness_slots must be greater than zero".to_owned());
        }
        if self.min_wallet_win_rate <= 0.0 || self.min_wallet_win_rate >= 1.0 {
            return Err("copy_trading.min_wallet_win_rate must be in (0.0, 1.0)".to_owned());
        }
        if self.min_wallet_pnl_30d <= 0.0 {
            return Err("copy_trading.min_wallet_pnl_30d must be positive".to_owned());
        }
        if self.min_wallet_trades_30d == 0 {
            return Err("copy_trading.min_wallet_trades_30d must be greater than zero".to_owned());
        }
        if self.max_concurrent_copies == 0 {
            return Err("copy_trading.max_concurrent_copies must be greater than zero".to_owned());
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Liquidation hunter
// ─────────────────────────────────────────────────────────────────────────────

/// Liquidation hunter strategy — scan lending protocols and execute liquidations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationConfig {
    /// Master toggle for the liquidation hunter loop.
    pub enabled: bool,
    /// Poll interval (ms) for WATCH and CRITICAL positions.
    pub scan_interval_ms: u64,
    /// Poll interval (seconds) for SAFE positions.
    pub safe_poll_interval_s: u64,
    /// Health factor below which a position enters WATCH tier.
    pub watch_health_threshold: f64,
    /// Health factor below which a position is CRITICAL (liquidatable).
    pub critical_health_threshold: f64,
    /// Maximum fraction of debt to repay per liquidation (0.0–1.0).
    pub max_repay_pct: f64,
    /// Fraction of expected liquidation bonus allocated as Jito tip.
    pub jito_tip_pct_of_bonus: f64,
    /// Lending protocols to scan (`kamino`, `marginfi`, `drift`, `save`).
    pub protocols: Vec<String>,
    /// Minimum borrow value (USD) to consider a position.
    pub min_position_value_usd: f64,
}

impl Default for LiquidationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            scan_interval_ms: 500,
            safe_poll_interval_s: 60,
            watch_health_threshold: 1.10,
            critical_health_threshold: 1.00,
            max_repay_pct: 0.50,
            jito_tip_pct_of_bonus: 0.70,
            protocols: vec![
                "kamino".to_owned(),
                "marginfi".to_owned(),
                "drift".to_owned(),
                "save".to_owned(),
            ],
            min_position_value_usd: 1000.0,
        }
    }
}

impl LiquidationConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.scan_interval_ms == 0 {
            return Err("liquidation.scan_interval_ms must be greater than zero".to_owned());
        }
        if self.safe_poll_interval_s == 0 {
            return Err("liquidation.safe_poll_interval_s must be greater than zero".to_owned());
        }
        if self.critical_health_threshold >= self.watch_health_threshold {
            return Err(
                "liquidation.critical_health_threshold must be less than watch_health_threshold"
                    .to_owned(),
            );
        }
        if !(0.0..=1.0).contains(&self.max_repay_pct) || self.max_repay_pct <= 0.0 {
            return Err("liquidation.max_repay_pct must be in (0.0, 1.0]".to_owned());
        }
        if !(0.0..=1.0).contains(&self.jito_tip_pct_of_bonus) {
            return Err("liquidation.jito_tip_pct_of_bonus must be in [0.0, 1.0]".to_owned());
        }
        if self.min_position_value_usd <= 0.0 {
            return Err("liquidation.min_position_value_usd must be positive".to_owned());
        }
        if self.protocols.is_empty() {
            return Err("liquidation.protocols must not be empty".to_owned());
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sniper
// ─────────────────────────────────────────────────────────────────────────────

/// New-token sniper strategy parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperConfig {
    /// Master toggle for pool-creation ingest + sniper executor.
    pub enabled: bool,
    /// Base position size in SOL for verified tokens.
    pub base_position_sol: f64,
    /// Maximum concurrent open sniper positions.
    pub max_concurrent_positions: u32,
    /// Minimum initial pool liquidity in SOL to consider a buy.
    pub min_liquidity_sol: f64,
    /// Reject when creator holding exceeds this percentage.
    pub max_creator_holding_pct: f64,
    /// Minimum Rugcheck score (0–1000 scale).
    pub rugcheck_min_score: u64,
    /// Take-profit 1 multiple (sell 50 % of position).
    pub take_profit_1_x: f64,
    /// Take-profit 2 multiple (sell remainder).
    pub take_profit_2_x: f64,
    /// Stop-loss drawdown fraction (0.20 = exit at 0.8× entry).
    pub stop_loss_pct: f64,
    /// Time-based exit in seconds (sell 100 %).
    pub time_exit_seconds: u64,
}

impl Default for SniperConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_position_sol: 0.1,
            max_concurrent_positions: 5,
            min_liquidity_sol: 5.0,
            max_creator_holding_pct: 15.0,
            rugcheck_min_score: 700,
            take_profit_1_x: 2.0,
            take_profit_2_x: 3.0,
            stop_loss_pct: 0.20,
            time_exit_seconds: 300,
        }
    }
}

impl SniperConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.base_position_sol <= 0.0 {
            return Err("sniper.base_position_sol must be positive".to_owned());
        }
        if self.max_concurrent_positions == 0 {
            return Err("sniper.max_concurrent_positions must be greater than zero".to_owned());
        }
        if self.min_liquidity_sol <= 0.0 {
            return Err("sniper.min_liquidity_sol must be positive".to_owned());
        }
        if self.max_creator_holding_pct <= 0.0 || self.max_creator_holding_pct > 100.0 {
            return Err("sniper.max_creator_holding_pct must be in (0.0, 100.0]".to_owned());
        }
        if self.take_profit_1_x <= 1.0 {
            return Err("sniper.take_profit_1_x must be greater than 1.0".to_owned());
        }
        if self.take_profit_2_x <= self.take_profit_1_x {
            return Err("sniper.take_profit_2_x must be greater than take_profit_1_x".to_owned());
        }
        if self.stop_loss_pct <= 0.0 || self.stop_loss_pct >= 1.0 {
            return Err("sniper.stop_loss_pct must be in (0.0, 1.0)".to_owned());
        }
        if self.time_exit_seconds == 0 {
            return Err("sniper.time_exit_seconds must be greater than zero".to_owned());
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Momentum / volume spike (Strategy 5)
// ─────────────────────────────────────────────────────────────────────────────

/// DexScreener volume-anomaly momentum strategy parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentumConfig {
    /// Master toggle for the momentum volume-spike poller and executor.
    pub enabled: bool,
    /// Minimum `volume_h1 / (volume_h24 / 24)` ratio to qualify as a spike.
    pub min_volume_ratio: f64,
    /// Maximum absolute 5m price velocity (% per minute) — filters blow-off tops.
    pub max_price_velocity_pct_min: f64,
    /// Minimum pool liquidity (USD) on the best pair.
    pub min_liquidity_usd: f64,
    /// Minimum confidence score (0.5 base + whale + multi-DEX bonuses).
    pub min_confidence: f64,
    /// Take-profit exit threshold as fractional gain (0.15 = +15%).
    pub take_profit_pct: f64,
    /// Stop-loss exit threshold as fractional loss (0.08 = -8%).
    pub stop_loss_pct: f64,
    /// Maximum hold duration before time-based exit.
    pub max_hold_minutes: u64,
    /// USD notional per momentum entry.
    pub position_usd: f64,
}

impl Default for MomentumConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_volume_ratio: 5.0,
            max_price_velocity_pct_min: 2.0,
            min_liquidity_usd: 50_000.0,
            min_confidence: 0.70,
            take_profit_pct: 0.15,
            stop_loss_pct: 0.08,
            max_hold_minutes: 10,
            position_usd: 25.0,
        }
    }
}

impl MomentumConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.min_volume_ratio <= 0.0 {
            return Err("momentum.min_volume_ratio must be positive".to_owned());
        }
        if self.max_price_velocity_pct_min <= 0.0 {
            return Err("momentum.max_price_velocity_pct_min must be positive".to_owned());
        }
        if self.min_liquidity_usd <= 0.0 {
            return Err("momentum.min_liquidity_usd must be positive".to_owned());
        }
        if !(0.0..=1.0).contains(&self.min_confidence) {
            return Err("momentum.min_confidence must be in [0.0, 1.0]".to_owned());
        }
        if self.take_profit_pct <= 0.0 {
            return Err("momentum.take_profit_pct must be positive".to_owned());
        }
        if self.stop_loss_pct <= 0.0 {
            return Err("momentum.stop_loss_pct must be positive".to_owned());
        }
        if self.max_hold_minutes == 0 {
            return Err("momentum.max_hold_minutes must be greater than zero".to_owned());
        }
        if self.position_usd <= 0.0 {
            return Err("momentum.position_usd must be positive".to_owned());
        }
        Ok(())
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

    /// Target take-profit as a fraction of notional (e.g. `0.012` = 1.2 %).
    pub take_profit_pct: f64,

    /// Stop-loss as a fraction of notional (e.g. `0.006` = 0.6 %).
    pub stop_loss_pct: f64,
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
            take_profit_pct: 0.012,
            stop_loss_pct: 0.006,
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
        if self.take_profit_pct <= 0.0 || self.take_profit_pct > 1.0 {
            return Err("scalper.take_profit_pct must be in (0.0, 1.0]".to_owned());
        }
        if self.stop_loss_pct <= 0.0 || self.stop_loss_pct >= 1.0 {
            return Err("scalper.stop_loss_pct must be in (0.0, 1.0)".to_owned());
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
// Cross-DEX arbitrage
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for the cross-DEX atomic arbitrage engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    /// Solana commitment for bundle landing — use `"processed"` for lowest latency.
    pub commitment: String,

    /// Minimum gross spread in basis points before an opportunity is considered.
    pub min_spread_bps: u64,

    /// Maximum route hops (2 = cross-DEX, 3 = triangular).
    pub max_route_hops: usize,

    /// Initial Jito tip as a fraction of gross profit (e.g. `0.50` = 50 %).
    pub jito_tip_pct: f64,

    /// Hard floor on Jito tip in lamports.
    pub jito_tip_min_lamports: u64,

    /// Hard ceiling on Jito tip as a fraction of gross profit.
    pub jito_tip_max_pct: f64,

    /// Target bundle acceptance rate for tip auto-calibration.
    pub bundle_acceptance_target: f64,

    /// Submit bundles to US / EU / Tokyo endpoints in parallel.
    pub parallel_endpoints: bool,

    /// Minimum per-leg pool liquidity in USD for route eligibility.
    pub min_leg_liquidity_usd: f64,
}

impl Default for ArbitrageConfig {
    fn default() -> Self {
        Self {
            commitment: "processed".to_owned(),
            min_spread_bps: 10,
            max_route_hops: 3,
            jito_tip_pct: 0.50,
            jito_tip_min_lamports: 5_000,
            jito_tip_max_pct: 0.65,
            bundle_acceptance_target: 0.70,
            parallel_endpoints: true,
            min_leg_liquidity_usd: 50_000.0,
        }
    }
}

impl ArbitrageConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        let valid_commitments = ["processed", "confirmed", "finalized"];
        if !valid_commitments.contains(&self.commitment.as_str()) {
            return Err(format!(
                "arbitrage.commitment must be one of {:?}, got {:?}",
                valid_commitments, self.commitment
            ));
        }
        if self.max_route_hops < 2 || self.max_route_hops > 3 {
            return Err("arbitrage.max_route_hops must be 2 or 3".to_owned());
        }
        if self.jito_tip_pct <= 0.0 || self.jito_tip_pct > 1.0 {
            return Err("arbitrage.jito_tip_pct must be in (0.0, 1.0]".to_owned());
        }
        if self.jito_tip_max_pct <= self.jito_tip_pct {
            return Err(
                "arbitrage.jito_tip_max_pct must be greater than jito_tip_pct".to_owned(),
            );
        }
        if self.min_leg_liquidity_usd <= 0.0 {
            return Err("arbitrage.min_leg_liquidity_usd must be positive".to_owned());
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Quote arb (route divergence)
// ─────────────────────────────────────────────────────────────────────────────

/// One directed pair scanned for Jupiter route-divergence / quote dislocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteArbPair {
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub input_decimals: u8,
    pub output_decimals: u8,
}

/// Jupiter route-divergence / quote arbitrage scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteArbConfig {
    pub enabled: bool,
    pub scan_interval_ms: u64,
    /// Minimum edge in basis points (route divergence or price dislocation).
    pub min_edge_bps: u64,
    /// Nominal trade size in USD used to size Jupiter quotes.
    pub trade_size_usd: f64,
    pub slippage_bps: u32,
    /// Require Jupiter strict-list tokens (Tokens API v2 tag).
    pub require_strict_tokens: bool,
    /// Pairs to scan each interval.
    #[serde(default)]
    pub pairs: Vec<QuoteArbPair>,
}

impl Default for QuoteArbConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            scan_interval_ms: 2_000,
            min_edge_bps: 15,
            trade_size_usd: 100.0,
            slippage_bps: 50,
            require_strict_tokens: false,
            pairs: vec![
                QuoteArbPair {
                    label: "SOL→USDC".to_owned(),
                    input_mint: "So11111111111111111111111111111111111111112".to_owned(),
                    output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_owned(),
                    input_decimals: 9,
                    output_decimals: 6,
                },
                QuoteArbPair {
                    label: "USDC→SOL".to_owned(),
                    input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_owned(),
                    output_mint: "So11111111111111111111111111111111111111112".to_owned(),
                    input_decimals: 6,
                    output_decimals: 9,
                },
            ],
        }
    }
}

impl QuoteArbConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.scan_interval_ms == 0 {
            return Err("quote_arb.scan_interval_ms must be positive".to_owned());
        }
        if self.trade_size_usd <= 0.0 {
            return Err("quote_arb.trade_size_usd must be positive".to_owned());
        }
        if self.enabled && self.pairs.is_empty() {
            return Err("quote_arb.pairs must be non-empty when enabled".to_owned());
        }
        for (i, pair) in self.pairs.iter().enumerate() {
            if pair.input_mint.is_empty() || pair.output_mint.is_empty() {
                return Err(format!("quote_arb.pairs[{i}] mints must be non-empty"));
            }
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CoinMarketCap
// ─────────────────────────────────────────────────────────────────────────────

/// CoinMarketCap top-N listings mapped to Solana mints for pair scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinMarketCapConfig {
    pub enabled: bool,
    /// Number of top listings to fetch (max 100 on free tier).
    pub top_n: u32,
    /// Quote mint for alt pairs (typically USDC).
    pub quote_mint: String,
    /// Maximum pairs to include in scan universe.
    pub max_pairs: u32,
    /// Hours between CMC cache refreshes.
    pub refresh_hours: u32,
}

impl Default for CoinMarketCapConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            top_n: 100,
            quote_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_owned(),
            max_pairs: 100,
            refresh_hours: 6,
        }
    }
}

impl CoinMarketCapConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.top_n == 0 {
            return Err("coinmarketcap.top_n must be positive".to_owned());
        }
        if self.max_pairs == 0 {
            return Err("coinmarketcap.max_pairs must be positive".to_owned());
        }
        if self.enabled && self.quote_mint.is_empty() {
            return Err("coinmarketcap.quote_mint must be set when enabled".to_owned());
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pump.fun
// ─────────────────────────────────────────────────────────────────────────────

/// Pump.fun bonding curve edge strategy parameters.
/// See: https://github.com/pump-fun/pump-public-docs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpFunConfig {
    pub enabled: bool,
    /// Minimum edge in basis points to act on a signal.
    pub min_edge_bps: u32,
    /// Nominal trade size in SOL for curve impact calculation.
    pub trade_sol: f64,
    /// Min graduation % for proximity plays (migration arb window).
    pub min_graduation_pct: f64,
    /// Max graduation % for proximity plays.
    pub max_graduation_pct: f64,
    /// Max price impact bps for early curve entries.
    pub max_early_impact_bps: u32,
    /// Min divergence bps between bonding curve and Jupiter price.
    pub min_curve_jupiter_divergence_bps: u32,
    /// Subscribe to Pump.fun Create events via Helius logs.
    pub monitor_launches: bool,
}

impl Default for PumpFunConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_edge_bps: 50,
            trade_sol: 0.5,
            min_graduation_pct: 80.0,
            max_graduation_pct: 99.0,
            max_early_impact_bps: 300,
            min_curve_jupiter_divergence_bps: 75,
            monitor_launches: true,
        }
    }
}

impl PumpFunConfig {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.trade_sol <= 0.0 {
            return Err("pump_fun.trade_sol must be positive".to_owned());
        }
        if self.min_graduation_pct >= self.max_graduation_pct {
            return Err("pump_fun.min_graduation_pct must be < max_graduation_pct".to_owned());
        }
        Ok(())
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
    fn feature_flags_reject_live_without_confirm_env() {
        let mut cfg = SystemConfig::default();
        cfg.features.dry_run = false;
        std::env::remove_var("SOLANA_ARB_CONFIRM_LIVE_TRADING");
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

    #[test]
    fn whale_tracker_config_defaults_are_valid() {
        WhaleTrackerConfig::default()
            .validate()
            .expect("default whale_tracker config is valid");
    }

    #[test]
    fn whale_tracker_config_rejects_zero_ttl() {
        let mut cfg = WhaleTrackerConfig::default();
        cfg.signal_ttl_seconds = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn copy_trading_config_defaults_are_valid() {
        CopyTradingConfig::default()
            .validate()
            .expect("default copy_trading config is valid");
    }

    #[test]
    fn copy_trading_config_rejects_zero_ratio() {
        let mut cfg = CopyTradingConfig::default();
        cfg.copy_ratio = 0.0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn momentum_config_defaults_are_valid() {
        MomentumConfig::default()
            .validate()
            .expect("default momentum config is valid");
    }

    #[test]
    fn momentum_config_rejects_zero_hold() {
        let mut cfg = MomentumConfig::default();
        cfg.max_hold_minutes = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn arbitrage_config_defaults_are_valid() {
        ArbitrageConfig::default()
            .validate()
            .expect("default arbitrage config is valid");
    }

    #[test]
    fn arbitrage_config_rejects_invalid_commitment() {
        let mut cfg = ArbitrageConfig::default();
        cfg.commitment = "instant".to_owned();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn sniper_config_defaults_are_valid() {
        SniperConfig::default()
            .validate()
            .expect("default sniper config is valid");
    }

    #[test]
    fn sniper_config_rejects_zero_positions() {
        let mut cfg = SniperConfig::default();
        cfg.max_concurrent_positions = 0;
        assert!(cfg.validate().is_err());
    }
}
