//! Concrete `SignalProcessor` implementations.

pub mod momentum;
pub mod smart_money;
pub mod whale;

pub use momentum::MomentumProcessor;
pub use smart_money::SmartMoneyProcessor;
pub use whale::WhaleFlowProcessor;
