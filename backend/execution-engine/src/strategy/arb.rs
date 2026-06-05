//! DEX-to-DEX arbitrage strategy stub.

use super::{ExecutionStrategy, SwapOrder};
use crate::signals::TradeSignal;

pub struct ArbStrategy;

impl ExecutionStrategy for ArbStrategy {
    fn name(&self) -> &'static str {
        "arb"
    }

    fn evaluate(&self, signal: &TradeSignal) -> bool {
        signal.expected_edge >= 8.0 && signal.confidence >= 0.6
    }

    fn build_order(&self, signal: &TradeSignal) -> Option<SwapOrder> {
        if !self.evaluate(signal) {
            return None;
        }
        let lamports = ((signal.size_usd / 145.0) * 1_000_000_000.0) as u64;
        Some(SwapOrder {
            input_mint: signal.token_in.clone(),
            output_mint: signal.token_out.clone(),
            amount_lamports: lamports.max(5_000_000),
            slippage_bps: 30,
            strategy: self.name().into(),
        })
    }
}
