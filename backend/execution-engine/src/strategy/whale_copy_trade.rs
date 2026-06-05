use super::{
    adjust_size, ExecutionStrategy, ExecutionTiming, RejectReason, SizedOrder,
};
use crate::config::EngineConfig;
use crate::signals::TradeSignal;

pub struct WhaleCopyTradeStrategy;

impl ExecutionStrategy for WhaleCopyTradeStrategy {
    fn name(&self) -> &'static str {
        "whale_copy_trade"
    }

    fn min_confidence(&self) -> f64 {
        0.65
    }

    fn validate(&self, signal: &TradeSignal) -> Result<(), RejectReason> {
        if signal.confidence < self.min_confidence() {
            return Err(RejectReason::LowConfidence);
        }
        if !signal.wallet.starts_with("whale_") && signal.size_usd < 50.0 {
            return Err(RejectReason::LowEdge);
        }
        Ok(())
    }

    fn size_position(&self, signal: &TradeSignal, config: &EngineConfig) -> SizedOrder {
        let mut sized = adjust_size(signal, config, 40);
        sized.size_usd *= 0.8;
        sized.amount_lamports = super::usd_to_lamports(sized.size_usd);
        sized
    }

    fn timing(&self, _signal: &TradeSignal) -> ExecutionTiming {
        ExecutionTiming::Immediate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_whale_or_large_size() {
        let s = WhaleCopyTradeStrategy;
        let small = TradeSignal::new("retail", "A", "B", 0.9, 20.0, 10.0, "whale_copy_trade");
        assert_eq!(s.validate(&small), Err(RejectReason::LowEdge));
        let whale = TradeSignal::new("whale_x", "A", "B", 0.9, 20.0, 10.0, "whale_copy_trade");
        assert!(s.validate(&whale).is_ok());
    }
}
