//! Market graph boundary.
//!
//! This crate will own token-market connectivity used by routing.

#![forbid(unsafe_code)]

pub mod market_graph {
    //! Market graph storage and update responsibilities.

    use common::Result;

    /// Placeholder market graph handle.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct MarketGraph;

    impl MarketGraph {
        /// Creates an empty placeholder graph.
        #[must_use]
        pub const fn new() -> Self {
            Self
        }

        /// Performs a no-op validation for the scaffold.
        pub const fn validate(&self) -> Result<()> {
            Ok(())
        }
    }
}

pub use market_graph::MarketGraph;

#[cfg(test)]
mod tests {
    use super::MarketGraph;

    #[test]
    fn graph_placeholder_validates() {
        assert!(MarketGraph::new().validate().is_ok());
    }
}
