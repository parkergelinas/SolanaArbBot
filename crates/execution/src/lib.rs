//! Execution simulation boundary.
//!
//! This crate will simulate trade execution from candidate routes.
//! The [`hotpath`] module provides the ultra low-latency synchronous pipeline.

#![forbid(unsafe_code)]

pub mod hotpath;
pub mod jito;
pub mod jupiter_swap;
pub mod tip_calibrator;
pub mod simulator {
    //! Execution simulation and outcome modeling responsibilities.

    use common::{Error, Result};
    use graph::Edge;
    use routing::{Route, Router};
    use tracing::debug;

    const BPS_DENOMINATOR: f64 = 10_000.0;

    /// Deterministic route simulation model.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SimulationModel {
        /// Constant-product AMM approximation.
        ConstantProductAmm,
        /// Simplified CLMM model with concentrated-liquidity slippage curve.
        SimplifiedClmm,
    }

    /// Route execution simulation output.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct ExecutionResult {
        pub final_amount: f64,
        pub profit: f64,
        pub slippage: f64,
    }

    /// Execution simulation configuration.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct ExecutionConfig {
        min_liquidity: f64,
        max_input_liquidity_ratio: f64,
        clmm_slippage_multiplier: f64,
    }

    impl ExecutionConfig {
        /// Creates execution simulation configuration.
        pub fn new(
            min_liquidity: f64,
            max_input_liquidity_ratio: f64,
            clmm_slippage_multiplier: f64,
        ) -> Result<Self> {
            if !min_liquidity.is_finite() || min_liquidity < 0.0 {
                return Err(Error::InvalidState(
                    "execution min liquidity must be finite and non-negative".to_owned(),
                ));
            }
            if !max_input_liquidity_ratio.is_finite()
                || max_input_liquidity_ratio <= 0.0
                || max_input_liquidity_ratio > 1.0
            {
                return Err(Error::InvalidState(
                    "execution max input/liquidity ratio must be in (0, 1]".to_owned(),
                ));
            }
            if !clmm_slippage_multiplier.is_finite() || clmm_slippage_multiplier < 0.0 {
                return Err(Error::InvalidState(
                    "execution CLMM slippage multiplier must be finite and non-negative".to_owned(),
                ));
            }

            Ok(Self {
                min_liquidity,
                max_input_liquidity_ratio,
                clmm_slippage_multiplier,
            })
        }

        /// Returns minimum edge liquidity allowed for simulation.
        #[must_use]
        pub const fn min_liquidity(self) -> f64 {
            self.min_liquidity
        }

        /// Returns maximum input-to-liquidity ratio allowed per hop.
        #[must_use]
        pub const fn max_input_liquidity_ratio(self) -> f64 {
            self.max_input_liquidity_ratio
        }
    }

    impl Default for ExecutionConfig {
        fn default() -> Self {
            Self {
                min_liquidity: 1.0,
                max_input_liquidity_ratio: 0.25,
                clmm_slippage_multiplier: 0.35,
            }
        }
    }

    /// Execution simulator handle.
    #[derive(Clone, Debug)]
    pub struct ExecutionSimulator {
        router: Router,
        config: ExecutionConfig,
    }

    impl ExecutionSimulator {
        /// Creates an execution simulator with default configuration.
        #[must_use]
        pub fn new(router: Router) -> Self {
            Self::with_config(router, ExecutionConfig::default())
        }

        /// Creates an execution simulator with explicit configuration.
        #[must_use]
        pub const fn with_config(router: Router, config: ExecutionConfig) -> Self {
            Self { router, config }
        }

        /// Returns the router boundary used by execution.
        #[must_use]
        pub const fn router(&self) -> &Router {
            &self.router
        }

        /// Returns execution configuration.
        #[must_use]
        pub const fn config(&self) -> ExecutionConfig {
            self.config
        }

        /// Simulates a route with the constant-product AMM approximation.
        pub fn simulate_route(
            &self,
            route: &Route,
            initial_amount: f64,
        ) -> Result<ExecutionResult> {
            self.simulate_route_with_model(
                route,
                initial_amount,
                SimulationModel::ConstantProductAmm,
            )
        }

        /// Simulates a route with an explicit deterministic model.
        pub fn simulate_route_with_model(
            &self,
            route: &Route,
            initial_amount: f64,
            model: SimulationModel,
        ) -> Result<ExecutionResult> {
            validate_route(route, initial_amount, self.config)?;

            let no_slippage_amount = route.path.iter().fold(initial_amount, |amount, edge| {
                amount * edge.price * fee_multiplier(edge)
            });
            let mut amount = initial_amount;

            for edge in &route.path {
                amount = match model {
                    SimulationModel::ConstantProductAmm => simulate_amm_hop(amount, edge),
                    SimulationModel::SimplifiedClmm => {
                        simulate_clmm_hop(amount, edge, self.config.clmm_slippage_multiplier)
                    }
                };
                if !amount.is_finite() || amount <= 0.0 {
                    return Err(Error::InvalidState(
                        "route simulation produced invalid output amount".to_owned(),
                    ));
                }
            }

            let slippage = if no_slippage_amount > 0.0 {
                ((no_slippage_amount - amount) / no_slippage_amount).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let result = ExecutionResult {
                final_amount: amount,
                profit: amount - initial_amount,
                slippage,
            };

            debug!(
                ?model,
                hops = route.path.len(),
                initial_amount,
                final_amount = result.final_amount,
                profit = result.profit,
                slippage = result.slippage,
                "simulated route execution"
            );

            Ok(result)
        }

        /// Performs a no-op readiness check for the scaffold.
        pub const fn ready(&self) -> Result<()> {
            Ok(())
        }
    }

    fn validate_route(route: &Route, initial_amount: f64, config: ExecutionConfig) -> Result<()> {
        if route.path.len() < 2 {
            return Err(Error::InvalidState(
                "execution route must contain at least two hops".to_owned(),
            ));
        }
        if !initial_amount.is_finite() || initial_amount <= 0.0 {
            return Err(Error::InvalidState(
                "execution initial amount must be finite and positive".to_owned(),
            ));
        }

        let first = route.path.first().expect("route length checked");
        let last = route.path.last().expect("route length checked");
        if first.from != last.to {
            return Err(Error::InvalidState(
                "execution route must be a cycle".to_owned(),
            ));
        }

        for window in route.path.windows(2) {
            if window[0].to != window[1].from {
                return Err(Error::InvalidState(
                    "execution route contains non-contiguous hops".to_owned(),
                ));
            }
        }

        let mut amount = initial_amount;
        for edge in &route.path {
            validate_edge(edge, amount, config)?;
            amount *= edge.price * fee_multiplier(edge);
        }

        Ok(())
    }

    fn validate_edge(edge: &Edge, input_amount: f64, config: ExecutionConfig) -> Result<()> {
        if !edge.price.is_finite() || edge.price <= 0.0 {
            return Err(Error::InvalidState(
                "execution edge price must be finite and positive".to_owned(),
            ));
        }
        if !edge.liquidity.is_finite() || edge.liquidity < config.min_liquidity() {
            return Err(Error::InvalidState(
                "execution edge liquidity is too low".to_owned(),
            ));
        }
        if input_amount / edge.liquidity > config.max_input_liquidity_ratio() {
            return Err(Error::InvalidState(
                "execution input amount is unrealistic for edge liquidity".to_owned(),
            ));
        }
        if fee_multiplier(edge) <= 0.0 {
            return Err(Error::InvalidState(
                "execution edge fee is too high".to_owned(),
            ));
        }

        Ok(())
    }

    fn simulate_amm_hop(input_amount: f64, edge: &Edge) -> f64 {
        let effective_input = input_amount * fee_multiplier(edge);
        let reserve_in = edge.liquidity;
        let reserve_out = edge.liquidity * edge.price;

        reserve_out * effective_input / (reserve_in + effective_input)
    }

    fn simulate_clmm_hop(input_amount: f64, edge: &Edge, slippage_multiplier: f64) -> f64 {
        let base_output = input_amount * edge.price * fee_multiplier(edge);
        let utilization = (input_amount / edge.liquidity).clamp(0.0, 1.0);
        let slippage_penalty = (utilization * utilization * slippage_multiplier).clamp(0.0, 0.95);

        base_output * (1.0 - slippage_penalty)
    }

    fn fee_multiplier(edge: &Edge) -> f64 {
        1.0 - (edge.fee_bps as f64 / BPS_DENOMINATOR)
    }
}

pub use hotpath::{
    ColdPathExecutor, ExecutionIntent, ExecutionRouter, HotPathEngine, HotPathStats,
    HotSignal, HotState, MarketTick, PrecomputeTable, RiskVerdict, RouteChoice, TickOutcome,
    Venue,
};
pub use jito::{
    build_bundle_signed, build_tip_tx_message, fetch_blockhash_blocking,
    submit_bundle_blocking_raw, wrap_signed_tx, BuiltBundle, BundleRequest,
    BundleSubmitResult, JitoSubmitter, COMPUTE_UNITS_LIMIT, JITO_COMMITMENT, JITO_ENDPOINTS,
};
pub use simulator::{ExecutionConfig, ExecutionResult, ExecutionSimulator, SimulationModel};
pub use jupiter_swap::{
    build_legacy_quote_url, execute_swap_blocking, sign_legacy_transaction,
    BlockingSwapTx, JupiterSwapConfig, JupiterSwapExecutor, SwapExecutionResult,
};
pub use tip_calibrator::TipCalibrator;

#[cfg(test)]
mod tests {
    use super::{ExecutionConfig, ExecutionSimulator, SimulationModel};
    use common::{Pubkey, Token};
    use graph::Edge;
    use routing::{Route, Router};

    #[test]
    fn execution_simulator_placeholder_is_ready() {
        let simulator = ExecutionSimulator::new(Router::default());

        assert!(simulator.ready().is_ok());
        assert!(simulator.router().graph().validate().is_ok());
    }

    #[test]
    fn simulates_profitable_amm_route_after_fees_and_slippage() {
        let simulator = ExecutionSimulator::new(Router::default());
        let route = Route {
            path: vec![
                edge(token(1), token(2), 2.0, 100_000.0, 25),
                edge(token(2), token(1), 0.55, 100_000.0, 25),
            ],
            score: 1.0,
        };

        let result = simulator.simulate_route(&route, 100.0).expect("simulate");

        assert!(result.final_amount > 100.0);
        assert!(result.profit > 0.0);
        assert!(result.slippage > 0.0);
    }

    #[test]
    fn thin_liquidity_is_penalized_heavily() {
        let simulator = ExecutionSimulator::new(Router::default());
        let deep = Route {
            path: vec![
                edge(token(1), token(2), 2.0, 100_000.0, 25),
                edge(token(2), token(1), 0.55, 100_000.0, 25),
            ],
            score: 1.0,
        };
        let thin = Route {
            path: vec![
                edge(token(1), token(2), 2.0, 1_000.0, 25),
                edge(token(2), token(1), 0.55, 1_000.0, 25),
            ],
            score: 1.0,
        };

        let deep_result = simulator.simulate_route(&deep, 100.0).expect("deep");
        let thin_result = simulator.simulate_route(&thin, 100.0).expect("thin");

        assert!(thin_result.profit < deep_result.profit);
        assert!(thin_result.slippage > deep_result.slippage);
    }

    #[test]
    fn rejects_unrealistic_input_for_liquidity() {
        let simulator = ExecutionSimulator::new(Router::default());
        let route = Route {
            path: vec![
                edge(token(1), token(2), 2.0, 100.0, 25),
                edge(token(2), token(1), 0.55, 100.0, 25),
            ],
            score: 1.0,
        };

        let err = simulator
            .simulate_route(&route, 100.0)
            .expect_err("unrealistic route");

        assert!(matches!(err, common::Error::InvalidState(_)));
    }

    #[test]
    fn rejects_non_cycle_route() {
        let simulator = ExecutionSimulator::new(Router::default());
        let route = Route {
            path: vec![
                edge(token(1), token(2), 2.0, 100_000.0, 25),
                edge(token(2), token(3), 0.55, 100_000.0, 25),
            ],
            score: 1.0,
        };

        let err = simulator
            .simulate_route(&route, 100.0)
            .expect_err("non-cycle");

        assert!(matches!(err, common::Error::InvalidState(_)));
    }

    #[test]
    fn simulates_simplified_clmm_route() {
        let config = ExecutionConfig::new(1.0, 0.5, 0.2).expect("config");
        let simulator = ExecutionSimulator::with_config(Router::default(), config);
        let route = Route {
            path: vec![
                edge(token(1), token(2), 1.5, 50_000.0, 20),
                edge(token(2), token(1), 0.75, 50_000.0, 20),
            ],
            score: 1.0,
        };

        let result = simulator
            .simulate_route_with_model(&route, 100.0, SimulationModel::SimplifiedClmm)
            .expect("simulate clmm");

        assert!(result.final_amount > 100.0);
        assert!(result.slippage > 0.0);
    }

    fn edge(from: Token, to: Token, price: f64, liquidity: f64, fee_bps: u64) -> Edge {
        Edge::new(from, to, price, liquidity, fee_bps)
    }

    fn token(byte: u8) -> Token {
        Token::new(Pubkey::new([byte; 32]), 6, None)
    }
}
