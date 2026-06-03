//! Routing boundary.
//!
//! This crate will evaluate candidate routes using graph topology and prices.

#![forbid(unsafe_code)]

pub mod router {
    //! Route search and scoring responsibilities.

    use graph::MarketGraph;
    use pricing::PricingEngine;

    /// Placeholder route planner handle.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Router {
        graph: MarketGraph,
        pricing: PricingEngine,
    }

    impl Router {
        /// Creates a placeholder router.
        #[must_use]
        pub const fn new(graph: MarketGraph, pricing: PricingEngine) -> Self {
            Self { graph, pricing }
        }

        /// Returns the graph boundary used by routing.
        #[must_use]
        pub const fn graph(&self) -> MarketGraph {
            self.graph
        }

        /// Returns the pricing boundary used by routing.
        #[must_use]
        pub const fn pricing(&self) -> PricingEngine {
            self.pricing
        }
    }
}

pub use router::Router;

#[cfg(test)]
mod tests {
    use super::Router;
    use graph::MarketGraph;
    use pricing::PricingEngine;

    #[test]
    fn router_holds_graph_and_pricing_boundaries() {
        let router = Router::new(MarketGraph::new(), PricingEngine::default());

        assert!(router.graph().validate().is_ok());
        assert!(router.pricing().ready().is_ok());
    }
}
