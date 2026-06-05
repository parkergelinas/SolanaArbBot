//! Data Transfer Objects — serde-serialisable versions of internal types.
//!
//! All internal crate types (Pubkey, SignalEvent, etc.) are converted here into
//! plain JSON-friendly structs before leaving the API boundary.

use serde::{Deserialize, Serialize};
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
        }
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
}

// ─────────────────────────────────────────────────────────────────────────────
// System state
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemStateDto {
    pub running: bool,
    /// Always "paper" — live mode is disabled in this build.
    pub mode: String,
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
