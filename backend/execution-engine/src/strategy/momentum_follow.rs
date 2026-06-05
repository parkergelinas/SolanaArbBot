use super::{
    adjust_size, ExecutionStrategy, ExecutionTiming, RejectReason, SizedOrder,
};
use crate::config::EngineConfig;
use crate::signals::TradeSignal;

pub struct MomentumFollowStrategy;

impl ExecutionStrategy for MomentumFollowStrategy {
    fn name(&self) -> &'static str {
        "momentum_follow"
    }

    fn min_confidence(&self) -> f64 {
        0.55
    }

    fn validate(&self, signal: &TradeSignal) -> Result<(), RejectReason> {
        if signal.confidence < self.min_confidence() {
            return Err(RejectReason::LowConfidence);
        }
        if signal.expected_edge < 5.0 {
            return Err(RejectReason::LowEdge);
        }
        Ok(())
    }

    fn size_position(&self, signal: &TradeSignal, config: &EngineConfig) -> SizedOrder {
        adjust_size(signal, config, 50)
    }

    fn timing(&self, signal: &TradeSignal) -> ExecutionTiming {
        if signal.confidence >= 0.8 {
            ExecutionTiming::Immediate
        } else {
            ExecutionTiming::DelayedMs(100)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EngineConfig;

    #[test]
    fn validate_and_size() {
        let s = MomentumFollowStrategy;
        let cfg = EngineConfig::from_env();
        let sig = TradeSignal::new("w", "SOL", "USDC", 0.7, 10.0, 100.0, "momentum_follow");
        assert!(s.validate(&sig).is_ok());
        let sized = s.size_position(&sig, &cfg);
        assert!(sized.size_usd <= cfg.max_position_usd);
        assert!(matches!(s.timing(&sig), ExecutionTiming::DelayedMs(_)));
    }
}
