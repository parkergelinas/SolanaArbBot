//! Strategy runtime controller — policy layer between config/UI and execution.
//!
//! Each autonomous loop tick asks [`StrategyController::plan_cycle`] for an
//! [`IngestionPlan`] and [`TradePolicy`].  Strategy selection drives real
//! runtime behaviour (ingestion source, signal filtering, trade eligibility).

use config::{StrategyConfig, SystemConfig};
use serde::{Deserialize, Serialize};
use signals::{SignalEvent, SignalType};
use tracing::info;

// ─────────────────────────────────────────────────────────────────────────────
// Runtime modes
// ─────────────────────────────────────────────────────────────────────────────

/// High-level bot lifecycle mode exposed to status APIs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeMode {
    /// Bot stopped — no ingestion or trading.
    Disabled,
    /// Explicit paper baseline — synthetic market events only.
    Paper,
    /// Strategy-driven mode — external / signal-bus ingestion (no fake ticks).
    Active,
    /// Running but all strategy toggles off — monitor only.
    Paused,
}

impl RuntimeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Paper => "paper",
            Self::Active => "active",
            Self::Paused => "paused",
        }
    }
}

/// Per-cycle ingestion source selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionMode {
    /// Synthetic `tick_events` — paper/test only.
    Synthetic,
    /// Internal signal-bus / buffered external market events.
    SignalBus,
    /// Intelligence-api whale alert bridge.
    Intelligence,
    /// No market ingestion this cycle.
    None,
}

impl IngestionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Synthetic => "synthetic",
            Self::SignalBus => "signal_bus",
            Self::Intelligence => "intelligence",
            Self::None => "none",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-cycle plans
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct IngestionPlan {
    pub mode: IngestionMode,
    /// Inject synthetic whale events (paper path only).
    pub inject_synthetic_whales: bool,
}

#[derive(Clone, Debug)]
pub struct TradePolicy {
    pub trading_enabled: bool,
    pub scalp_enabled: bool,
    pub arb_enabled: bool,
    pub whale_copy: bool,
    pub momentum: bool,
    pub sniper: bool,
    pub dry_run: bool,
    pub min_confidence: f64,
    pub min_strength: f64,
}

impl TradePolicy {
    /// Deterministic signal filter — whale-copy vs momentum produce different eligibility.
    pub fn signal_eligible(&self, signal: &SignalEvent) -> bool {
        if !self.trading_enabled {
            return false;
        }
        if signal.confidence < self.min_confidence || signal.strength < self.min_strength {
            return false;
        }

        match signal.signal_type {
            SignalType::WhaleFlow | SignalType::SmartMoney => {
                if self.whale_copy {
                    return true;
                }
                if self.momentum {
                    // Momentum mode ignores whale-only unless flow persistence filter passes.
                    return momentum_persistence_filter(signal);
                }
                if self.sniper {
                    return signal.strength >= 0.7;
                }
                false
            }
            SignalType::Momentum => {
                if self.momentum {
                    return momentum_persistence_filter(signal);
                }
                if self.whale_copy {
                    // Whale-copy may still act on strong momentum when whale activity coexists.
                    return signal.feature_vector.whale_activity_score >= 0.4
                        || signal.feature_vector.smart_money_score >= 0.5;
                }
                if self.sniper {
                    return signal.strength >= 0.75;
                }
                false
            }
        }
    }

    /// Rank for trade selection when multiple signals compete (lower = higher priority).
    pub fn signal_priority(&self, signal: &SignalEvent) -> u8 {
        if !self.signal_eligible(signal) {
            return u8::MAX;
        }
        match signal.signal_type {
            SignalType::WhaleFlow if self.whale_copy => 0,
            SignalType::SmartMoney if self.whale_copy => 1,
            SignalType::Momentum if self.momentum => 2,
            SignalType::Momentum => 3,
            SignalType::WhaleFlow | SignalType::SmartMoney => 4,
        }
    }
}

fn momentum_persistence_filter(signal: &SignalEvent) -> bool {
    let fv = &signal.feature_vector;
    fv.price_velocity.abs() >= 0.001
        || (fv.volume_short > 0.0 && fv.volume_long > 0.0 && fv.volume_short / fv.volume_long >= 1.05)
}

#[derive(Clone, Debug)]
pub struct CyclePlan {
    pub runtime_mode: RuntimeMode,
    pub ingestion: IngestionPlan,
    pub trade: TradePolicy,
    pub active_strategies: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Controller
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct StrategyTransition {
    pub from_mode: RuntimeMode,
    pub to_mode: RuntimeMode,
    pub active_strategies: Vec<String>,
    pub ingestion_mode: IngestionMode,
}

/// Thread-safe strategy policy holder (wrap in `Arc<RwLock<_>>` at the API boundary).
#[derive(Clone, Debug)]
pub struct StrategyController {
    running: bool,
    selection: StrategyConfig,
    last_runtime_mode: RuntimeMode,
    last_ingestion_mode: IngestionMode,
}

impl Default for StrategyController {
    fn default() -> Self {
        Self::from_selection(StrategyConfig::default(), false)
    }
}

impl StrategyController {
    pub fn from_config(cfg: &SystemConfig, running: bool) -> Self {
        Self::from_selection(cfg.strategy.clone(), running)
    }

    pub fn from_selection(selection: StrategyConfig, running: bool) -> Self {
        let ctrl = Self {
            running,
            selection,
            last_runtime_mode: RuntimeMode::Disabled,
            last_ingestion_mode: IngestionMode::None,
        };
        ctrl
    }

    pub fn selection(&self) -> &StrategyConfig {
        &self.selection
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn set_running(&mut self, running: bool) -> Option<StrategyTransition> {
        self.running = running;
        self.transition_if_changed()
    }

    pub fn apply_selection(&mut self, selection: StrategyConfig) -> Option<StrategyTransition> {
        if self.selection == selection {
            return None;
        }
        self.selection = selection;
        self.transition_if_changed()
    }

    pub fn sync_from_config(&mut self, cfg: &SystemConfig) -> Option<StrategyTransition> {
        self.apply_selection(cfg.strategy.clone())
    }

    fn transition_if_changed(&mut self) -> Option<StrategyTransition> {
        let plan = self.plan_cycle_inner(true);
        if plan.runtime_mode == self.last_runtime_mode
            && plan.ingestion.mode == self.last_ingestion_mode
        {
            return None;
        }
        let transition = StrategyTransition {
            from_mode: self.last_runtime_mode,
            to_mode: plan.runtime_mode,
            active_strategies: plan.active_strategies.clone(),
            ingestion_mode: plan.ingestion.mode,
        };
        self.last_runtime_mode = plan.runtime_mode;
        self.last_ingestion_mode = plan.ingestion.mode;
        info!(
            from = transition.from_mode.as_str(),
            to = transition.to_mode.as_str(),
            ingestion = transition.ingestion_mode.as_str(),
            strategies = ?transition.active_strategies,
            "strategy runtime mode transition"
        );
        Some(transition)
    }

    pub fn plan_cycle(&self, cfg: &SystemConfig) -> CyclePlan {
        self.plan_cycle_inner(false)
            .with_thresholds(cfg)
    }

    fn plan_cycle_inner(&self, _for_transition: bool) -> CyclePlan {
        let active = active_strategy_names(&self.selection);
        let runtime_mode = resolve_runtime_mode(self.running, &self.selection);

        let ingestion = match runtime_mode {
            RuntimeMode::Disabled | RuntimeMode::Paused => IngestionPlan {
                mode: IngestionMode::None,
                inject_synthetic_whales: false,
            },
            RuntimeMode::Paper => IngestionPlan {
                mode: IngestionMode::Synthetic,
                inject_synthetic_whales: true,
            },
            RuntimeMode::Active => {
                let mode = if self.selection.whale_copy {
                    IngestionMode::Intelligence
                } else {
                    IngestionMode::SignalBus
                };
                IngestionPlan {
                    mode,
                    inject_synthetic_whales: false,
                }
            }
        };

        let trading_enabled = self.running && !self.selection.all_disabled();
        let trade = TradePolicy {
            trading_enabled,
            scalp_enabled: trading_enabled && self.selection.scalp,
            arb_enabled: trading_enabled && self.selection.arb,
            whale_copy: self.selection.whale_copy,
            momentum: self.selection.momentum,
            sniper: self.selection.sniper,
            dry_run: true,
            min_confidence: 0.0,
            min_strength: 0.0,
        };

        CyclePlan {
            runtime_mode,
            ingestion,
            trade,
            active_strategies: active,
        }
    }
}

impl CyclePlan {
    fn with_thresholds(mut self, cfg: &SystemConfig) -> Self {
        self.trade.min_confidence = cfg.signal_engine.signal_min_confidence;
        self.trade.min_strength = cfg.signal_engine.signal_min_strength;
        self.trade.dry_run = cfg.features.dry_run;
        self
    }
}

fn resolve_runtime_mode(running: bool, selection: &StrategyConfig) -> RuntimeMode {
    if !running {
        return RuntimeMode::Disabled;
    }
    if selection.all_disabled() {
        return RuntimeMode::Paused;
    }
    if selection.requires_live_ingestion() {
        RuntimeMode::Active
    } else {
        RuntimeMode::Paper
    }
}

fn active_strategy_names(sel: &StrategyConfig) -> Vec<String> {
    let mut out = Vec::new();
    if sel.scalp {
        out.push("scalp".to_owned());
    }
    if sel.arb {
        out.push("arb".to_owned());
    }
    if sel.whale_copy {
        out.push("whale_copy".to_owned());
    }
    if sel.momentum {
        out.push("momentum".to_owned());
    }
    if sel.sniper {
        out.push("sniper".to_owned());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Pubkey;
    use signals::{Direction, FeatureVector};

    fn sample_signal(signal_type: SignalType, whale_score: f64, price_vel: f64) -> SignalEvent {
        SignalEvent {
            signal_id: 1,
            timestamp_micros: 1,
            pool_address: Pubkey::new([1; 32]),
            signal_type,
            strength: 0.6,
            confidence: 0.7,
            direction: Direction::Long,
            timeframe_secs: 60,
            feature_vector: FeatureVector {
                volume_short: 100.0,
                volume_long: 90.0,
                price_velocity: price_vel,
                liquidity_delta_pct: 0.0,
                whale_activity_score: whale_score,
                smart_money_score: whale_score,
                data_points: 10,
            },
            explanation: "test".to_owned(),
        }
    }

    fn policy_whale_copy() -> TradePolicy {
        TradePolicy {
            trading_enabled: true,
            scalp_enabled: true,
            arb_enabled: true,
            whale_copy: true,
            momentum: false,
            sniper: false,
            dry_run: true,
            min_confidence: 0.05,
            min_strength: 0.05,
        }
    }

    fn policy_momentum() -> TradePolicy {
        TradePolicy {
            trading_enabled: true,
            scalp_enabled: true,
            arb_enabled: true,
            whale_copy: false,
            momentum: true,
            sniper: false,
            dry_run: true,
            min_confidence: 0.05,
            min_strength: 0.05,
        }
    }

    #[test]
    fn whale_copy_accepts_whale_flow_rejects_bare_momentum() {
        let policy = policy_whale_copy();
        let whale = sample_signal(SignalType::WhaleFlow, 0.9, 0.0);
        let momentum = sample_signal(SignalType::Momentum, 0.0, 0.002);
        assert!(policy.signal_eligible(&whale));
        assert!(!policy.signal_eligible(&momentum));
    }

    #[test]
    fn momentum_rejects_whale_only_without_persistence() {
        let policy = policy_momentum();
        let mut whale_only = sample_signal(SignalType::WhaleFlow, 0.9, 0.0);
        whale_only.feature_vector.volume_short = 50.0;
        whale_only.feature_vector.volume_long = 100.0;
        let momentum = sample_signal(SignalType::Momentum, 0.0, 0.002);
        assert!(!policy.signal_eligible(&whale_only));
        assert!(policy.signal_eligible(&momentum));
    }

    #[test]
    fn paper_mode_only_when_no_live_strategies() {
        let mut ctrl = StrategyController::default();
        ctrl.set_running(true);
        let cfg = SystemConfig::default();
        let plan = ctrl.plan_cycle(&cfg);
        assert_eq!(plan.runtime_mode, RuntimeMode::Paper);
        assert_eq!(plan.ingestion.mode, IngestionMode::Synthetic);

        ctrl.apply_selection(StrategyConfig {
            whale_copy: true,
            ..StrategyConfig::default()
        });
        let plan = ctrl.plan_cycle(&cfg);
        assert_eq!(plan.runtime_mode, RuntimeMode::Active);
        assert_eq!(plan.ingestion.mode, IngestionMode::Intelligence);
        assert!(!plan.ingestion.inject_synthetic_whales);
    }

    #[test]
    fn disabled_when_stopped() {
        let ctrl = StrategyController::from_selection(StrategyConfig::default(), false);
        let plan = ctrl.plan_cycle(&SystemConfig::default());
        assert_eq!(plan.runtime_mode, RuntimeMode::Disabled);
        assert_eq!(plan.ingestion.mode, IngestionMode::None);
    }

    #[test]
    fn whale_copy_higher_priority_than_momentum() {
        let policy = TradePolicy {
            whale_copy: true,
            momentum: true,
            ..policy_whale_copy()
        };
        let whale = sample_signal(SignalType::WhaleFlow, 0.9, 0.0);
        let momentum = sample_signal(SignalType::Momentum, 0.0, 0.002);
        assert!(policy.signal_priority(&whale) < policy.signal_priority(&momentum));
    }
}
