//! Pluggable execution strategies.

pub mod arbitrage_capture;
pub mod momentum_follow;
pub mod registry;
pub mod sniper_entry;
pub mod whale_copy_trade;

use serde::{Deserialize, Serialize};

use crate::config::EngineConfig;
use crate::signals::TradeSignal;

pub use arbitrage_capture::ArbitrageCaptureStrategy;
pub use momentum_follow::MomentumFollowStrategy;
pub use registry::StrategyRegistry;
pub use sniper_entry::SniperEntryStrategy;
pub use whale_copy_trade::WhaleCopyTradeStrategy;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    LowConfidence,
    LowEdge,
    StrategyDisabled,
    TimingSkip,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionTiming {
    Immediate,
    DelayedMs(u64),
    Skip,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SizedOrder {
    pub input_mint: String,
    pub output_mint: String,
    pub amount_lamports: u64,
    pub size_usd: f64,
    pub slippage_bps: u32,
    pub strategy: String,
}

pub trait ExecutionStrategy: Send + Sync {
    fn name(&self) -> &'static str;
    fn min_confidence(&self) -> f64;
    fn validate(&self, signal: &TradeSignal) -> Result<(), RejectReason>;
    fn size_position(&self, signal: &TradeSignal, config: &EngineConfig) -> SizedOrder;
    fn timing(&self, signal: &TradeSignal) -> ExecutionTiming;
}

pub fn usd_to_lamports(size_usd: f64) -> u64 {
    let sol = size_usd / 145.0;
    (sol * 1_000_000_000.0).max(1_000_000.0) as u64
}

pub fn adjust_size(signal: &TradeSignal, config: &EngineConfig, slippage_bps: u32) -> SizedOrder {
    let scale = signal.confidence.clamp(0.0, 1.0) * (1.0 + signal.expected_edge / 100.0);
    let size_usd = (signal.size_usd * scale).min(config.max_position_usd);
    let (input_mint, output_mint) = signal.resolve_mints(config);
    SizedOrder {
        input_mint,
        output_mint,
        amount_lamports: usd_to_lamports(size_usd),
        size_usd,
        slippage_bps,
        strategy: signal.strategy.clone(),
    }
}
