//! Risk boundary.
//!
//! This crate will own pre-trade and simulation risk checks.

#![forbid(unsafe_code)]

pub mod checks {
    //! Risk check composition and evaluation responsibilities.

    use common::{Error, Result};
    use execution::ExecutionResult;
    use routing::Route;
    use tracing::{debug, warn};

    /// Route risk decision.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RiskDecision {
        pub allowed: bool,
        pub reason: Option<String>,
    }

    impl RiskDecision {
        /// Creates an allowed decision.
        #[must_use]
        pub fn allowed() -> Self {
            Self {
                allowed: true,
                reason: None,
            }
        }

        /// Creates a rejected decision with a reason.
        #[must_use]
        pub fn rejected(reason: impl Into<String>) -> Self {
            Self {
                allowed: false,
                reason: Some(reason.into()),
            }
        }
    }

    /// Rule-based risk configuration.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct RiskConfig {
        min_liquidity: f64,
        max_slippage: f64,
        max_depth: usize,
        min_price_product: f64,
        max_price_product: f64,
    }

    impl RiskConfig {
        /// Creates risk configuration.
        pub fn new(
            min_liquidity: f64,
            max_slippage: f64,
            max_depth: usize,
            min_price_product: f64,
            max_price_product: f64,
        ) -> Result<Self> {
            if !min_liquidity.is_finite() || min_liquidity < 0.0 {
                return Err(Error::InvalidState(
                    "risk min liquidity must be finite and non-negative".to_owned(),
                ));
            }
            if !max_slippage.is_finite() || !(0.0..=1.0).contains(&max_slippage) {
                return Err(Error::InvalidState(
                    "risk max slippage must be in [0, 1]".to_owned(),
                ));
            }
            if max_depth < 2 {
                return Err(Error::InvalidState(
                    "risk max depth must be at least 2".to_owned(),
                ));
            }
            if !min_price_product.is_finite()
                || !max_price_product.is_finite()
                || min_price_product <= 0.0
                || max_price_product < min_price_product
            {
                return Err(Error::InvalidState(
                    "risk price product bounds are invalid".to_owned(),
                ));
            }

            Ok(Self {
                min_liquidity,
                max_slippage,
                max_depth,
                min_price_product,
                max_price_product,
            })
        }

        /// Minimum per-edge liquidity required.
        #[must_use]
        pub const fn min_liquidity(self) -> f64 {
            self.min_liquidity
        }

        /// Maximum tolerated route slippage.
        #[must_use]
        pub const fn max_slippage(self) -> f64 {
            self.max_slippage
        }

        /// Maximum route depth in hops.
        #[must_use]
        pub const fn max_depth(self) -> usize {
            self.max_depth
        }
    }

    impl Default for RiskConfig {
        fn default() -> Self {
            Self {
                min_liquidity: 1_000.0,
                max_slippage: 0.05,
                max_depth: 3,
                min_price_product: 0.2,
                max_price_product: 5.0,
            }
        }
    }

    /// Rule-based risk engine.
    #[derive(Clone, Copy, Debug)]
    pub struct RiskEngine {
        config: RiskConfig,
    }

    impl RiskEngine {
        /// Creates a risk engine with default configuration.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                config: RiskConfig {
                    min_liquidity: 1_000.0,
                    max_slippage: 0.05,
                    max_depth: 3,
                    min_price_product: 0.2,
                    max_price_product: 5.0,
                },
            }
        }

        /// Creates a risk engine with explicit configuration.
        #[must_use]
        pub const fn with_config(config: RiskConfig) -> Self {
            Self { config }
        }

        /// Returns risk configuration.
        #[must_use]
        pub const fn config(&self) -> RiskConfig {
            self.config
        }

        /// Applies rule-based filters to a route and its deterministic execution result.
        #[must_use]
        pub fn evaluate_route(&self, route: &Route, execution: &ExecutionResult) -> RiskDecision {
            if route.path.is_empty() {
                return reject("route is empty");
            }
            if route.path.len() > self.config.max_depth {
                return reject("route depth exceeds risk limit");
            }

            let mut price_product = 1.0;
            for edge in &route.path {
                if !edge.price.is_finite() || edge.price <= 0.0 {
                    return reject("unstable pricing: edge price is invalid");
                }
                if edge.liquidity < self.config.min_liquidity {
                    return reject("liquidity too low");
                }
                price_product *= edge.price;
            }

            if !price_product.is_finite()
                || price_product < self.config.min_price_product
                || price_product > self.config.max_price_product
            {
                return reject("unstable pricing: route price product outside bounds");
            }

            if !execution.slippage.is_finite() || execution.slippage > self.config.max_slippage {
                return reject("slippage too high");
            }
            if !execution.final_amount.is_finite() || !execution.profit.is_finite() {
                return reject("execution result is invalid");
            }

            debug!(
                hops = route.path.len(),
                profit = execution.profit,
                slippage = execution.slippage,
                "risk decision allowed route"
            );
            RiskDecision::allowed()
        }

        /// Performs a readiness check for scaffold wiring.
        pub const fn ready(&self) -> Result<()> {
            Ok(())
        }
    }

    impl Default for RiskEngine {
        fn default() -> Self {
            Self::new()
        }
    }

    fn reject(reason: &'static str) -> RiskDecision {
        warn!(reason, "risk decision rejected route");
        RiskDecision::rejected(reason)
    }
}

pub use checks::{RiskConfig, RiskDecision, RiskEngine};

#[cfg(test)]
mod tests {
    use super::{RiskConfig, RiskDecision, RiskEngine};
    use common::{Pubkey, Token};
    use execution::ExecutionResult;
    use graph::Edge;
    use routing::Route;

    #[test]
    fn risk_engine_placeholder_is_ready() {
        assert!(RiskEngine::new().ready().is_ok());
    }

    #[test]
    fn allows_route_within_risk_limits() {
        let decision = RiskEngine::new().evaluate_route(
            &route(vec![
                edge(token(1), token(2), 1.2, 10_000.0),
                edge(token(2), token(1), 0.9, 10_000.0),
            ]),
            &execution(105.0, 5.0, 0.02),
        );

        assert_eq!(decision, RiskDecision::allowed());
    }

    #[test]
    fn rejects_low_liquidity_route() {
        let decision = RiskEngine::new().evaluate_route(
            &route(vec![
                edge(token(1), token(2), 1.2, 500.0),
                edge(token(2), token(1), 0.9, 10_000.0),
            ]),
            &execution(105.0, 5.0, 0.02),
        );

        assert_rejected_for(decision, "liquidity");
    }

    #[test]
    fn rejects_high_slippage() {
        let decision = RiskEngine::new().evaluate_route(
            &route(vec![
                edge(token(1), token(2), 1.2, 10_000.0),
                edge(token(2), token(1), 0.9, 10_000.0),
            ]),
            &execution(101.0, 1.0, 0.2),
        );

        assert_rejected_for(decision, "slippage");
    }

    #[test]
    fn rejects_unstable_pricing() {
        let decision = RiskEngine::new().evaluate_route(
            &route(vec![
                edge(token(1), token(2), 10.0, 10_000.0),
                edge(token(2), token(1), 10.0, 10_000.0),
            ]),
            &execution(105.0, 5.0, 0.02),
        );

        assert_rejected_for(decision, "pricing");
    }

    #[test]
    fn rejects_excessive_depth() {
        let decision =
            RiskEngine::with_config(RiskConfig::new(1_000.0, 0.05, 2, 0.2, 5.0).unwrap())
                .evaluate_route(
                    &route(vec![
                        edge(token(1), token(2), 1.1, 10_000.0),
                        edge(token(2), token(3), 1.1, 10_000.0),
                        edge(token(3), token(1), 0.9, 10_000.0),
                    ]),
                    &execution(105.0, 5.0, 0.02),
                );

        assert_rejected_for(decision, "depth");
    }

    fn assert_rejected_for(decision: RiskDecision, text: &str) {
        assert!(!decision.allowed);
        assert!(decision.reason.expect("reason").contains(text));
    }

    fn route(path: Vec<Edge>) -> Route {
        Route { path, score: 1.0 }
    }

    fn edge(from: Token, to: Token, price: f64, liquidity: f64) -> Edge {
        Edge::new(from, to, price, liquidity, 25)
    }

    fn execution(final_amount: f64, profit: f64, slippage: f64) -> ExecutionResult {
        ExecutionResult {
            final_amount,
            profit,
            slippage,
        }
    }

    fn token(byte: u8) -> Token {
        Token::new(Pubkey::new([byte; 32]), 6, None)
    }
}
