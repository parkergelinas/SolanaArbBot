use super::{
    ExecutionStrategy, ExecutionTiming, RejectReason, SizedOrder,
};
use crate::config::EngineConfig;
use crate::signals::TradeSignal;

pub struct ArbitrageCaptureStrategy;

impl ExecutionStrategy for ArbitrageCaptureStrategy {
    fn name(&self) -> &'static str {
        "arbitrage_capture"
    }

    fn min_confidence(&self) -> f64 {
        0.7
    }

    fn validate(&self, _signal: &TradeSignal) -> Result<(), RejectReason> {
        Err(RejectReason::StrategyDisabled)
    }

    fn size_position(&self, signal: &TradeSignal, config: &EngineConfig) -> SizedOrder {
        super::adjust_size(signal, config, 30)
    }

    fn timing(&self, _signal: &TradeSignal) -> ExecutionTiming {
        ExecutionTiming::Skip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_not_implemented() {
        let s = ArbitrageCaptureStrategy;
        let sig = TradeSignal::new("w", "A", "B", 0.9, 30.0, 100.0, "arbitrage_capture");
        assert_eq!(s.validate(&sig), Err(RejectReason::StrategyDisabled));
        assert_eq!(s.timing(&sig), ExecutionTiming::Skip);
    }
}
