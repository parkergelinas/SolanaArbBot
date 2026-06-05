use super::{
    adjust_size, ExecutionStrategy, ExecutionTiming, RejectReason, SizedOrder,
};
use crate::config::EngineConfig;
use crate::signals::TradeSignal;

pub struct SniperEntryStrategy;

impl ExecutionStrategy for SniperEntryStrategy {
    fn name(&self) -> &'static str {
        "sniper_entry"
    }

    fn min_confidence(&self) -> f64 {
        0.75
    }

    fn validate(&self, signal: &TradeSignal) -> Result<(), RejectReason> {
        if signal.confidence < self.min_confidence() {
            return Err(RejectReason::LowConfidence);
        }
        if signal.expected_edge < 15.0 {
            return Err(RejectReason::LowEdge);
        }
        Ok(())
    }

    fn size_position(&self, signal: &TradeSignal, config: &EngineConfig) -> SizedOrder {
        let mut sized = adjust_size(signal, config, 60);
        sized.size_usd = sized.size_usd.min(config.max_position_usd * 0.5);
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
    fn high_edge_required() {
        let s = SniperEntryStrategy;
        let low = TradeSignal::new("SOL", "long", 0.9, 5.0, 50.0, "sniper_entry");
        assert_eq!(s.validate(&low), Err(RejectReason::LowEdge));
    }
}
