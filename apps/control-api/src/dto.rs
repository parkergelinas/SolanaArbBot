//! Data Transfer Objects — serde-serialisable versions of internal types.
//!
//! All internal crate types (Pubkey, SignalEvent, etc.) are converted here into
//! plain JSON-friendly structs before leaving the API boundary.

use serde::{Deserialize, Serialize};
use signal_bus::{AlertType, LiveSignal, SignalKind, StrategyTag};
use signals::{Direction, FeatureVector, SignalEvent, SignalType};

// ─────────────────────────────────────────────────────────────────────────────
// Feature vector
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureVectorDto {
    pub volume_short: f64,
    pub volume_long: f64,
    pub price_velocity: f64,
    pub liquidity_delta_pct: f64,
    pub whale_activity_score: f64,
    pub smart_money_score: f64,
    pub data_points: usize,
}

impl From<&FeatureVector> for FeatureVectorDto {
    fn from(fv: &FeatureVector) -> Self {
        Self {
            volume_short: fv.volume_short,
            volume_long: fv.volume_long,
            price_velocity: fv.price_velocity,
            liquidity_delta_pct: fv.liquidity_delta_pct,
            whale_activity_score: fv.whale_activity_score,
            smart_money_score: fv.smart_money_score,
            data_points: fv.data_points,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Signal event
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalEventDto {
    pub signal_id: u64,
    pub timestamp_micros: u64,
    /// Pool address as a lowercase hex string.
    pub pool_address: String,
    pub signal_type: String,
    pub strength: f64,
    pub confidence: f64,
    pub direction: String,
    pub timeframe_secs: u64,
    pub feature_vector: FeatureVectorDto,
    pub explanation: String,
    /// Ingestion source — `engine`, `intelligence`, or `stream`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Bot routing hint — `whale_copy_candidate`, `watch_only`, `informational`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_tag: Option<String>,
    /// Wallet address when sourced from intelligence alerts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_sol: Option<f64>,
}

impl From<&SignalEvent> for SignalEventDto {
    fn from(s: &SignalEvent) -> Self {
        let pool_hex: String = s
            .pool_address
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        let signal_type = match s.signal_type {
            SignalType::WhaleFlow => "WhaleFlow",
            SignalType::SmartMoney => "SmartMoney",
            SignalType::Momentum => "Momentum",
        }
        .to_owned();

        let direction = match s.direction {
            Direction::Long => "Long",
            Direction::Short => "Short",
            Direction::Neutral => "Neutral",
        }
        .to_owned();

        Self {
            signal_id: s.signal_id,
            timestamp_micros: s.timestamp_micros,
            pool_address: pool_hex,
            signal_type,
            strength: s.strength,
            confidence: s.confidence,
            direction,
            timeframe_secs: s.timeframe_secs,
            feature_vector: FeatureVectorDto::from(&s.feature_vector),
            explanation: s.explanation.clone(),
            source: Some("engine".to_owned()),
            strategy_tag: None,
            wallet: None,
            size_usd: None,
            size_sol: None,
        }
    }
}

impl From<&LiveSignal> for SignalEventDto {
    fn from(s: &LiveSignal) -> Self {
        let strength = s.strength.unwrap_or(s.confidence);
        let signal_type = s
            .alert_type
            .map(AlertType::as_api_str)
            .unwrap_or_else(|| match s.kind {
                SignalKind::Swap => "Swap",
                SignalKind::WhaleAlert => "WhaleFlow",
                SignalKind::SmartMoneyAlert => "SmartMoney",
                SignalKind::Engine => "Momentum",
            })
            .to_owned();

        let mut fv = FeatureVectorDto {
            volume_short: s.size,
            volume_long: 0.0,
            price_velocity: 0.0,
            liquidity_delta_pct: 0.0,
            whale_activity_score: 0.0,
            smart_money_score: 0.0,
            data_points: 1,
        };
        match s.alert_type {
            Some(AlertType::WhaleFlow) => fv.whale_activity_score = strength,
            Some(AlertType::SmartMoney) => fv.smart_money_score = s.confidence,
            _ => {}
        }

        Self {
            signal_id: s.numeric_id(),
            timestamp_micros: s.timestamp_micros(),
            pool_address: s.token_out.clone(),
            signal_type,
            strength,
            confidence: s.confidence,
            direction: s.direction.clone().unwrap_or_else(|| "Long".into()),
            timeframe_secs: 300,
            feature_vector: fv,
            explanation: s
                .explanation
                .clone()
                .unwrap_or_else(|| format!("{} {}", s.pair, s.tx_id)),
            source: Some(s.source.layer.clone()),
            strategy_tag: s.strategy_tag.map(strategy_tag_str).map(str::to_owned),
            wallet: if s.wallet.is_empty() {
                None
            } else {
                Some(s.wallet.clone())
            },
            size_usd: s.size_usd,
            size_sol: Some(s.size),
        }
    }
}

fn strategy_tag_str(t: StrategyTag) -> &'static str {
    match t {
        StrategyTag::WhaleCopyCandidate => "whale_copy_candidate",
        StrategyTag::WatchOnly => "watch_only",
        StrategyTag::Informational => "informational",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Health
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthDto {
    pub status: String,
    pub uptime_secs: u64,
    pub signals_stored: usize,
    pub version: String,
    /// Whether the autonomous bot loop is active (`/api/system/start`).
    #[serde(default)]
    pub bot_running: bool,
    /// `development` | `staging` | `production` from `DEPLOY_ENV`.
    #[serde(default)]
    pub deploy_env: String,
    /// `paper` or `live` derived from feature flags.
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub events_processed: u64,
    /// Named readiness checks (`ok` / `degraded` / `missing`).
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub checks: std::collections::HashMap<String, String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// System state
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemStateDto {
    pub running: bool,
    /// `disabled` | `paper` | `active` | `paused`
    pub mode: String,
    pub runtime_mode: String,
    pub ingestion_mode: String,
    pub active_strategies: Vec<String>,
    pub signals_processed: u64,
    pub events_processed: u64,
    pub last_signal_ts: Option<u64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Portfolio (placeholder — wired to paper trading engine later)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioDto {
    pub capital_usd: f64,
    pub unrealised_pnl: f64,
    pub realised_pnl: f64,
    pub open_positions: usize,
    pub total_trades: u64,
    pub win_rate: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Risk
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskDto {
    pub capital_usd: f64,
    pub max_position_pct: f64,
    pub max_drawdown_pct: f64,
    pub current_exposure_pct: f64,
    pub daily_loss_usd: f64,
    pub risk_status: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Config patch (accepted on PATCH /api/config)
// ─────────────────────────────────────────────────────────────────────────────

/// Partial update for signal engine thresholds.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SignalConfigPatch {
    pub whale_threshold_usd: Option<f64>,
    pub momentum_window_secs: Option<u64>,
    pub smart_money_min_score: Option<f64>,
    pub signal_min_strength: Option<f64>,
    pub signal_min_confidence: Option<f64>,
    pub cooldown_secs: Option<u64>,
}

/// Partial update for feature flags.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FeatureFlagsPatch {
    pub dry_run: Option<bool>,
    pub enable_momentum_signals: Option<bool>,
    pub enable_whale_signals: Option<bool>,
    pub enable_smart_money_signals: Option<bool>,
}

/// Top-level config patch accepted by `PATCH /api/config`.
/// All fields are optional — only supplied fields are applied.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ConfigPatch {
    pub signal_engine: Option<SignalConfigPatch>,
    pub features: Option<FeatureFlagsPatch>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Trade lifecycle
// ─────────────────────────────────────────────────────────────────────────────

/// Lifecycle stage for a trade execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeStageDto {
    Started,
    Quoted,
    Validated,
    Submitted,
    Filled,
    Failed,
    Rejected,
    Canceled,
}

/// Paper vs live execution mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeModeDto {
    Paper,
    Live,
}

/// Trade side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeSideDto {
    Long,
    Short,
    Buy,
    Sell,
}

/// Canonical trade lifecycle event — mirrors `shared/contracts/trade/v1.ts`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeEventDto {
    pub v: u32,
    pub trade_id: String,
    pub wallet_id: String,
    pub source_strategy: String,
    pub pair: String,
    pub side: TradeSideDto,
    pub size_usd: f64,
    pub expected_pnl_usd: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_signature: Option<String>,
    pub timestamp_us: u64,
    pub stage: TradeStageDto,
    pub mode: TradeModeDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_id: Option<u64>,
}

impl TradeEventDto {
    pub const SCHEMA_VERSION: u32 = 1;
}

impl From<&autonomous::TradeEmit> for TradeEventDto {
    fn from(t: &autonomous::TradeEmit) -> Self {
        let side = match t.side.as_str() {
            "short" => TradeSideDto::Short,
            "buy" => TradeSideDto::Buy,
            "sell" => TradeSideDto::Sell,
            _ => TradeSideDto::Long,
        };
        let stage = match t.stage {
            autonomous::TradeStage::Started => TradeStageDto::Started,
            autonomous::TradeStage::Quoted => TradeStageDto::Quoted,
            autonomous::TradeStage::Validated => TradeStageDto::Validated,
            autonomous::TradeStage::Submitted => TradeStageDto::Submitted,
            autonomous::TradeStage::Filled => TradeStageDto::Filled,
            autonomous::TradeStage::Failed => TradeStageDto::Failed,
            autonomous::TradeStage::Rejected => TradeStageDto::Rejected,
            autonomous::TradeStage::Canceled => TradeStageDto::Canceled,
        };
        let mode = if t.mode == "live" {
            TradeModeDto::Live
        } else {
            TradeModeDto::Paper
        };
        Self {
            v: Self::SCHEMA_VERSION,
            trade_id: t.trade_id.clone(),
            wallet_id: t.wallet_id.clone(),
            source_strategy: t.source_strategy.clone(),
            pair: t.pair.clone(),
            side,
            size_usd: t.size_usd,
            expected_pnl_usd: t.expected_pnl_usd,
            tx_signature: t.tx_signature.clone(),
            timestamp_us: t.timestamp_us,
            stage,
            mode,
            reject_reason: t.reject_reason.clone(),
            signal_id: t.signal_id,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Control commands
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
}

impl CommandResult {
    pub fn ok(msg: impl Into<String>) -> Self {
        Self { success: true, message: msg.into() }
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self { success: false, message: msg.into() }
    }
}
