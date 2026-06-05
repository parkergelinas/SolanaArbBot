//! Risk gates — must pass before Jupiter quote.

use std::sync::Arc;

use dashmap::DashMap;

use crate::config::EngineConfig;
use crate::risk::volatility::VolatilityFilter;
use crate::signals::{TradeSignal, unix_ms};
use crate::strategy::{ExecutionStrategy, SizedOrder};

#[derive(Debug, Clone, PartialEq)]
pub enum RiskReject {
    Oversize { got: f64, max: f64 },
    LowConfidence { got: f64, min: f64 },
    Cooldown { token: String, secs_remaining: u64 },
    Volatility,
}

pub struct RiskGate {
    max_position_usd: f64,
    cooldown_secs: u64,
    cooldowns: DashMap<String, u64>,
    volatility: Arc<dyn VolatilityFilter>,
}

impl RiskGate {
    pub fn new(config: &EngineConfig, volatility: Arc<dyn VolatilityFilter>) -> Self {
        Self {
            max_position_usd: config.max_position_usd,
            cooldown_secs: config.token_cooldown_secs,
            cooldowns: DashMap::new(),
            volatility,
        }
    }

    pub fn evaluate(
        &self,
        signal: &TradeSignal,
        strategy: &dyn ExecutionStrategy,
        sized: &SizedOrder,
    ) -> Result<(), RiskReject> {
        let min_conf = strategy.min_confidence();
        if signal.confidence < min_conf {
            return Err(RiskReject::LowConfidence {
                got: signal.confidence,
                min: min_conf,
            });
        }

        if sized.size_usd > self.max_position_usd {
            return Err(RiskReject::Oversize {
                got: sized.size_usd,
                max: self.max_position_usd,
            });
        }

        if !self.volatility.allows(signal) {
            return Err(RiskReject::Volatility);
        }

        let now = unix_ms();
        if let Some(last) = self.cooldowns.get(&sized.output_mint) {
            let elapsed = now.saturating_sub(*last) / 1000;
            if elapsed < self.cooldown_secs {
                return Err(RiskReject::Cooldown {
                    token: sized.output_mint.clone(),
                    secs_remaining: self.cooldown_secs - elapsed,
                });
            }
        }

        Ok(())
    }

    pub fn record_fill(&self, token: &str) {
        self.cooldowns.insert(token.to_string(), unix_ms());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::volatility::NoOpVolatilityFilter;
    use crate::strategy::MomentumFollowStrategy;

    fn gate() -> RiskGate {
        let cfg = EngineConfig {
            paper_mode: true,
            execution_live: false,
            min_confidence: 0.55,
            min_edge_bps: 5.0,
            max_position_usd: 100.0,
            default_slippage_bps: 50,
            token_cooldown_secs: 30,
            request_timeout_ms: 800,
            jupiter_base_url: "https://quote-api.jup.ag".into(),
            audit_path: None,
            wallet_pubkey: "test".into(),
            quote_cache_ttl_ms: 200,
            quote_mint: "USDC".into(),
            base_mint: "SOL".into(),
        };
        RiskGate::new(&cfg, Arc::new(NoOpVolatilityFilter))
    }

    #[test]
    fn rejects_oversize() {
        let g = gate();
        let strat = MomentumFollowStrategy;
        let sig = TradeSignal::new("SOL", "long", 0.9, 20.0, 200.0, "momentum_follow");
        let sized = strat.size_position(&sig, &EngineConfig::from_env());
        let big = SizedOrder {
            size_usd: 150.0,
            ..sized
        };
        assert!(matches!(
            g.evaluate(&sig, &strat, &big),
            Err(RiskReject::Oversize { .. })
        ));
    }

    #[test]
    fn rejects_cooldown() {
        let g = gate();
        let strat = MomentumFollowStrategy;
        let sig = TradeSignal::new("USDC", "long", 0.9, 20.0, 50.0, "momentum_follow");
        let sized = SizedOrder {
            input_mint: "SOL".into(),
            output_mint: "USDC".into(),
            amount_lamports: 1_000_000,
            size_usd: 50.0,
            slippage_bps: 50,
            strategy: "momentum_follow".into(),
        };
        g.record_fill("USDC");
        assert!(matches!(
            g.evaluate(&sig, &strat, &sized),
            Err(RiskReject::Cooldown { .. })
        ));
    }
}
