//! Strategy registry keyed by TradeSignal.strategy name.

use std::collections::HashMap;
use std::sync::Arc;

use super::{
    ArbitrageCaptureStrategy, ExecutionStrategy, MomentumFollowStrategy, SniperEntryStrategy,
    WhaleCopyTradeStrategy,
};

pub struct StrategyRegistry {
    strategies: HashMap<String, Arc<dyn ExecutionStrategy>>,
}

impl StrategyRegistry {
    pub fn new() -> Self {
        let mut strategies: HashMap<String, Arc<dyn ExecutionStrategy>> = HashMap::new();
        let entries: Vec<Arc<dyn ExecutionStrategy>> = vec![
            Arc::new(MomentumFollowStrategy),
            Arc::new(WhaleCopyTradeStrategy),
            Arc::new(ArbitrageCaptureStrategy),
            Arc::new(SniperEntryStrategy),
        ];
        for s in entries {
            strategies.insert(s.name().to_string(), s);
        }
        Self { strategies }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn ExecutionStrategy>> {
        self.strategies.get(name).cloned()
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.strategies.values().map(|s| s.name()).collect()
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::TradeSignal;

    #[test]
    fn dispatches_momentum_follow() {
        let reg = StrategyRegistry::new();
        let s = reg.get("momentum_follow").expect("registered");
        assert_eq!(s.name(), "momentum_follow");
        let sig = TradeSignal::new("SOL", "long", 0.8, 15.0, 50.0, "momentum_follow");
        assert!(s.validate(&sig).is_ok());
    }

    #[test]
    fn dispatches_whale_copy_trade() {
        let reg = StrategyRegistry::new();
        assert!(reg.get("whale_copy_trade").is_some());
    }
}
