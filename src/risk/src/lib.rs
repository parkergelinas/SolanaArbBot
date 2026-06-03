//! Risk boundary.
//!
//! This crate will own pre-trade and simulation risk checks.

#![forbid(unsafe_code)]

pub mod checks {
    //! Risk check composition and evaluation responsibilities.

    use common::Result;

    /// Placeholder risk engine handle.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct RiskEngine;

    impl RiskEngine {
        /// Creates a placeholder risk engine.
        #[must_use]
        pub const fn new() -> Self {
            Self
        }

        /// Performs a no-op readiness check for the scaffold.
        pub const fn ready(&self) -> Result<()> {
            Ok(())
        }
    }
}

pub use checks::RiskEngine;

#[cfg(test)]
mod tests {
    use super::RiskEngine;

    #[test]
    fn risk_engine_placeholder_is_ready() {
        assert!(RiskEngine::new().ready().is_ok());
    }
}
