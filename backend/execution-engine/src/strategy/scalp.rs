//! Scalping strategy — momentum / short-hold swaps.

use super::{ExecutionStrategy, SwapOrder};
use crate::signals::TradeSignal;

pub struct ScalpStrategy;

impl ExecutionStrategy for ScalpStrategy {
    fn name(&self) -> &'static str {
        "scalp"
    }

    fn evaluate(&self, signal: &TradeSignal) -> bool {
        signal.expected_edge >= 3.0 && signal.confidence >= 0.5
    }

    fn build_order(&self, signal: &TradeSignal) -> Option<SwapOrder> {
        if !self.evaluate(signal) {
            return None;
        }
        let lamports = ((signal.size_usd / 145.0) * 1_000_000_000.0) as u64;
        Some(SwapOrder {
            input_mint: signal.token_in.clone(),
            output_mint: signal.token_out.clone(),
            amount_lamports: lamports.max(1_000_000),
            slippage_bps: 50,
            strategy: self.name().into(),
        })
    }
}
