use super::{
    adjust_size, ExecutionStrategy, ExecutionTiming, RejectReason, SizedOrder,
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
        adjust_size(signal, config, 30)
    }

    fn timing(&self, _signal: &TradeSignal) -> ExecutionTiming {
        ExecutionTiming::Immediate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_arb_signal() {
        let s = ArbitrageCaptureStrategy;
        let sig = TradeSignal::new("SOL", "long", 0.85, 30.0, 100.0, "arbitrage_capture");
        assert!(s.validate(&sig).is_ok());
        assert_eq!(s.timing(&sig), ExecutionTiming::Immediate);
    }
}
