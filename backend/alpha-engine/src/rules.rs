//! Signal generation gates — alpha, confidence, liquidity, conflicts.

use dashmap::DashMap;

use crate::config::AlphaConfig;
use crate::types::{AlphaSignal, FusedFeatureVector, TradeSignal, unix_ms};

#[derive(Debug, Clone, PartialEq)]
pub enum RejectReason {
    LowAlphaScore { got: f64, min: f64 },
    LowConfidence { got: f64, min: f64 },
    LowLiquidity { got: f64, min: f64 },
    Cooldown { token: String, strategy: String },
    ConflictingSignal { token: String, strategy: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowSide {
    Long,
    Short,
}

pub struct SignalGate {
    min_alpha_score: f64,
    min_confidence: f64,
    min_liquidity: f64,
    cooldown_secs: u64,
    recent: DashMap<(String, String), u64>,
    active_sides: DashMap<String, FlowSide>,
}

impl SignalGate {
    pub fn new(config: &AlphaConfig) -> Self {
        Self {
            min_alpha_score: config.min_alpha_score,
            min_confidence: config.min_confidence,
            min_liquidity: config.min_liquidity,
            cooldown_secs: config.cooldown_secs,
            recent: DashMap::new(),
            active_sides: DashMap::new(),
        }
    }

    pub fn evaluate(
        &self,
        fused: &FusedFeatureVector,
        alpha: &AlphaSignal,
    ) -> Result<TradeSignal, RejectReason> {
        if alpha.score < self.min_alpha_score {
            return Err(RejectReason::LowAlphaScore {
                got: alpha.score,
                min: self.min_alpha_score,
            });
        }
        if alpha.confidence < self.min_confidence {
            return Err(RejectReason::LowConfidence {
                got: alpha.confidence,
                min: self.min_confidence,
            });
        }
        if fused.liquidity_conditions < self.min_liquidity {
            return Err(RejectReason::LowLiquidity {
                got: fused.liquidity_conditions,
                min: self.min_liquidity,
            });
        }

        let side = if alpha.direction == "long" {
            FlowSide::Long
        } else {
            FlowSide::Short
        };

        if self.has_conflict(&alpha.token_out, &alpha.strategy, side) {
            return Err(RejectReason::ConflictingSignal {
                token: alpha.token_out.clone(),
                strategy: alpha.strategy.clone(),
            });
        }

        let now = unix_ms();
        let key = (alpha.token_out.clone(), alpha.strategy.clone());
        if let Some(last) = self.recent.get(&key) {
            let elapsed = now.saturating_sub(*last) / 1000;
            if elapsed < self.cooldown_secs {
                return Err(RejectReason::Cooldown {
                    token: alpha.token_out.clone(),
                    strategy: alpha.strategy.clone(),
                });
            }
        }

        self.recent.insert(key, now);
        self.active_sides.insert(alpha.token_out.clone(), side);
        Ok(TradeSignal::from_alpha(alpha))
    }

    pub fn has_conflict(&self, token: &str, strategy: &str, side: FlowSide) -> bool {
        if let Some(existing) = self.active_sides.get(token) {
            let opposing = matches!(
                (*existing, side),
                (FlowSide::Long, FlowSide::Short) | (FlowSide::Short, FlowSide::Long)
            );
            if opposing && strategy != "arbitrage_capture" {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AlphaConfig;

    fn fused(liq: f64) -> FusedFeatureVector {
        FusedFeatureVector {
            token: "SOL".into(),
            wallet_smart_money_score: 0.8,
            momentum_strength: 0.7,
            liquidity_conditions: liq,
            arbitrage_opportunity_strength: 0.5,
            volume_spike_norm: 0.6,
            price_change: 0.02,
            imbalance: 0.1,
            timestamp: unix_ms(),
            ttl_ms: 5000,
        }
    }

    fn alpha(score: f64) -> AlphaSignal {
        AlphaSignal {
            signal_id: "t".into(),
            token_in: "USDC".into(),
            token_out: "SOL".into(),
            wallet: "w".into(),
            confidence: score,
            expected_edge: 15.0,
            size_usd: 100.0,
            strategy: "momentum_follow".into(),
            score,
            direction: "long".into(),
            timestamp: unix_ms(),
        }
    }

    #[test]
    fn passes_when_all_gates_met() {
        let gate = SignalGate::new(&AlphaConfig::from_env());
        let result = gate.evaluate(&fused(0.6), &alpha(0.85));
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_low_alpha_score() {
        let gate = SignalGate::new(&AlphaConfig::from_env());
        let err = gate.evaluate(&fused(0.6), &alpha(0.5)).unwrap_err();
        assert!(matches!(err, RejectReason::LowAlphaScore { .. }));
    }

    #[test]
    fn rejects_low_liquidity() {
        let gate = SignalGate::new(&AlphaConfig::from_env());
        let err = gate.evaluate(&fused(0.1), &alpha(0.85)).unwrap_err();
        assert!(matches!(err, RejectReason::LowLiquidity { .. }));
    }

    #[test]
    fn rejects_cooldown() {
        let gate = SignalGate::new(&AlphaConfig::from_env());
        let f = fused(0.6);
        let a = alpha(0.85);
        assert!(gate.evaluate(&f, &a).is_ok());
        let err = gate.evaluate(&f, &a).unwrap_err();
        assert!(matches!(err, RejectReason::Cooldown { .. }));
    }

    #[test]
    fn rejects_conflicting_signals() {
        let gate = SignalGate::new(&AlphaConfig::from_env());
        gate.active_sides.insert("SOL".into(), FlowSide::Short);
        let mut a = alpha(0.85);
        a.direction = "long".into();
        let err = gate.evaluate(&fused(0.6), &a).unwrap_err();
        assert!(matches!(err, RejectReason::ConflictingSignal { .. }));
    }
}
