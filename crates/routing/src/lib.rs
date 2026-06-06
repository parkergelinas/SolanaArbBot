//! Routing boundary.
//!
//! This crate will evaluate candidate routes using graph topology and prices.

#![forbid(unsafe_code)]

pub mod score_adjust;

pub mod router {
    //! Route search and scoring responsibilities.

    use common::{Error, Result, Token};
    use graph::{Edge, MarketGraph};
    use pricing::PricingEngine;
    use signals::{ExternalSignalStore, WhaleSignalStore};
    use tracing::{debug, trace};

    use crate::score_adjust::apply_score_multipliers;

    const DEFAULT_MAX_DEPTH: usize = 3;
    const DEFAULT_DEPTH_PENALTY_BPS: u64 = 50;

    /// Candidate arbitrage cycle.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Route {
        pub path: Vec<Edge>,
        pub score: f64,
    }

    /// Routing search configuration.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct RoutingConfig {
        max_depth: usize,
        min_liquidity: f64,
        depth_penalty_bps: u64,
    }

    impl RoutingConfig {
        /// Creates routing configuration.
        pub fn new(max_depth: usize, min_liquidity: f64, depth_penalty_bps: u64) -> Result<Self> {
            if !(2..=DEFAULT_MAX_DEPTH).contains(&max_depth) {
                return Err(Error::InvalidState(
                    "routing max depth must be between 2 and 3".to_owned(),
                ));
            }
            if !min_liquidity.is_finite() || min_liquidity < 0.0 {
                return Err(Error::InvalidState(
                    "routing min liquidity must be finite and non-negative".to_owned(),
                ));
            }

            Ok(Self {
                max_depth,
                min_liquidity,
                depth_penalty_bps,
            })
        }

        /// Returns max DFS cycle depth in edges.
        #[must_use]
        pub const fn max_depth(self) -> usize {
            self.max_depth
        }

        /// Returns minimum edge liquidity allowed in candidate routes.
        #[must_use]
        pub const fn min_liquidity(self) -> f64 {
            self.min_liquidity
        }

        /// Returns per-hop depth penalty in basis points.
        #[must_use]
        pub const fn depth_penalty_bps(self) -> u64 {
            self.depth_penalty_bps
        }
    }

    impl Default for RoutingConfig {
        fn default() -> Self {
            Self {
                max_depth: DEFAULT_MAX_DEPTH,
                min_liquidity: 0.0,
                depth_penalty_bps: DEFAULT_DEPTH_PENALTY_BPS,
            }
        }
    }

    /// Route planner handle.
    #[derive(Clone, Debug, Default)]
    pub struct Router {
        graph: MarketGraph,
        pricing: PricingEngine,
        config: RoutingConfig,
        external: Option<ExternalSignalStore>,
        whale: Option<WhaleSignalStore>,
        whale_ttl_secs: u64,
    }

    impl Router {
        /// Creates a router with default routing configuration.
        #[must_use]
        pub fn new(graph: MarketGraph, pricing: PricingEngine) -> Self {
            Self::with_config(graph, pricing, RoutingConfig::default())
        }

        /// Creates a router with explicit routing configuration.
        #[must_use]
        pub fn with_config(
            graph: MarketGraph,
            pricing: PricingEngine,
            config: RoutingConfig,
        ) -> Self {
            Self {
                graph,
                pricing,
                config,
                external: None,
                whale: None,
                whale_ttl_secs: 60,
            }
        }

        /// Attach external signal context for route score multipliers.
        #[must_use]
        pub fn with_external_signals(mut self, store: ExternalSignalStore) -> Self {
            self.external = Some(store);
            self
        }

        /// Attach whale swap signal cache for route score boosts.
        #[must_use]
        pub fn with_whale_signals(mut self, store: WhaleSignalStore, ttl_secs: u64) -> Self {
            self.whale = Some(store);
            self.whale_ttl_secs = ttl_secs;
            self
        }

        /// Finds arbitrage-cycle candidates up to the configured depth.
        pub fn find_arbitrage_cycles(&self) -> Vec<Route> {
            let mut routes = Vec::new();
            let starts = self.graph.source_tokens().cloned().collect::<Vec<_>>();

            debug!(
                start_count = starts.len(),
                max_depth = self.config.max_depth(),
                min_liquidity = self.config.min_liquidity(),
                "starting arbitrage cycle search"
            );

            for start in starts {
                let mut path = Vec::with_capacity(self.config.max_depth());
                let mut visited = vec![start.clone()];
                self.search_from(&start, &start, &mut visited, &mut path, &mut routes);
            }

            routes.sort_by(|left, right| right.score.total_cmp(&left.score));
            routes
        }

        fn search_from(
            &self,
            start: &Token,
            current: &Token,
            visited: &mut Vec<Token>,
            path: &mut Vec<Edge>,
            routes: &mut Vec<Route>,
        ) {
            if path.len() >= self.config.max_depth() {
                return;
            }

            for edge in self
                .graph
                .outgoing_edges(current)
                .filter(|edge| edge.liquidity >= self.config.min_liquidity())
            {
                let next_depth = path.len() + 1;
                if &edge.to == start {
                    if next_depth >= 2 {
                        path.push(edge.clone());
                        let mut score = score_path(path, self.config.depth_penalty_bps());
                        if self.external.is_some() || self.whale.is_some() {
                            let mint = mint_str(&start);
                            let ext = self
                                .external
                                .as_ref()
                                .cloned()
                                .unwrap_or_default();
                            score = apply_score_multipliers(
                                score,
                                &mint,
                                &ext,
                                self.whale.as_ref(),
                                self.whale_ttl_secs,
                            );
                        }
                        if score <= 0.0 {
                            path.pop();
                            continue;
                        }
                        trace!(depth = next_depth, score, "found arbitrage cycle candidate");
                        routes.push(Route {
                            path: path.clone(),
                            score,
                        });
                        path.pop();
                    }
                    continue;
                }

                if next_depth >= self.config.max_depth()
                    || visited.iter().any(|token| token == &edge.to)
                {
                    continue;
                }

                path.push(edge.clone());
                visited.push(edge.to.clone());
                self.search_from(start, &edge.to, visited, path, routes);
                visited.pop();
                path.pop();
            }
        }

        /// Returns the graph boundary used by routing.
        #[must_use]
        pub const fn graph(&self) -> &MarketGraph {
            &self.graph
        }

        /// Returns the pricing boundary used by routing.
        #[must_use]
        pub const fn pricing(&self) -> PricingEngine {
            self.pricing
        }

        /// Returns routing configuration.
        #[must_use]
        pub const fn config(&self) -> RoutingConfig {
            self.config
        }
    }

    fn mint_str(token: &Token) -> String {
        bs58::encode(token.mint().as_bytes()).into_string()
    }

    fn score_path(path: &[Edge], depth_penalty_bps: u64) -> f64 {
        if path.is_empty() {
            return 0.0;
        }

        let bottleneck_liquidity = path
            .iter()
            .map(|edge| edge.liquidity)
            .fold(f64::INFINITY, f64::min);
        let price_product = path.iter().map(|edge| edge.price).product::<f64>();
        let fee_multiplier = path
            .iter()
            .map(|edge| 1.0 - (edge.fee_bps as f64 / 10_000.0))
            .product::<f64>();
        let depth_penalty = 1.0 + (path.len() as f64 * depth_penalty_bps as f64 / 10_000.0);

        bottleneck_liquidity * price_product * fee_multiplier / depth_penalty
    }
}

pub use router::{Route, Router, RoutingConfig};
pub use score_adjust::{apply_score_multipliers, apply_whale_boosts};

#[cfg(test)]
mod tests {
    use super::{Router, RoutingConfig};
    use common::{Pubkey, Token};
    use graph::{Edge, MarketGraph};
    use pricing::PricingEngine;

    #[test]
    fn router_holds_graph_and_pricing_boundaries() {
        let router = Router::new(MarketGraph::new(), PricingEngine::default());

        assert!(router.graph().validate().is_ok());
        assert!(router.pricing().ready().is_ok());
    }

    #[test]
    fn router_finds_two_hop_cycle() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let router = Router::new(
            graph_with_edges(vec![
                Edge::new(sol.clone(), usdc.clone(), 2.0, 1_000.0, 25),
                Edge::new(usdc.clone(), sol.clone(), 0.6, 900.0, 25),
            ]),
            PricingEngine::default(),
        );

        let routes = router.find_arbitrage_cycles();

        assert!(!routes.is_empty());
        assert!(routes.iter().any(|route| route.path.len() == 2));
        assert!(routes.iter().all(|route| route.score > 0.0));
    }

    #[test]
    fn router_finds_three_hop_cycle() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let usdt = token(3, "USDT");
        let router = Router::new(
            graph_with_edges(vec![
                Edge::new(sol.clone(), usdc.clone(), 2.0, 1_000.0, 25),
                Edge::new(usdc.clone(), usdt.clone(), 1.1, 950.0, 20),
                Edge::new(usdt.clone(), sol.clone(), 0.55, 900.0, 25),
            ]),
            PricingEngine::default(),
        );

        let routes = router.find_arbitrage_cycles();

        assert!(routes.iter().any(|route| route.path.len() == 3));
    }

    #[test]
    fn router_prunes_low_liquidity_edges() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let config = RoutingConfig::new(3, 500.0, 50).expect("config");
        let router = Router::with_config(
            graph_with_edges(vec![
                Edge::new(sol.clone(), usdc.clone(), 2.0, 1_000.0, 25),
                Edge::new(usdc.clone(), sol.clone(), 0.6, 100.0, 25),
            ]),
            PricingEngine::default(),
            config,
        );

        assert!(router.find_arbitrage_cycles().is_empty());
    }

    #[test]
    fn router_scores_lower_fee_and_higher_liquidity_better() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let rich_low_fee = graph_with_edges(vec![
            Edge::new(sol.clone(), usdc.clone(), 2.0, 2_000.0, 10),
            Edge::new(usdc.clone(), sol.clone(), 0.6, 2_000.0, 10),
        ]);
        let thin_high_fee = graph_with_edges(vec![
            Edge::new(sol.clone(), usdc.clone(), 2.0, 500.0, 100),
            Edge::new(usdc.clone(), sol.clone(), 0.6, 500.0, 100),
        ]);

        let rich_score = Router::new(rich_low_fee, PricingEngine::default())
            .find_arbitrage_cycles()
            .first()
            .expect("rich route")
            .score;
        let thin_score = Router::new(thin_high_fee, PricingEngine::default())
            .find_arbitrage_cycles()
            .first()
            .expect("thin route")
            .score;

        assert!(rich_score > thin_score);
    }

    fn graph_with_edges(edges: Vec<Edge>) -> MarketGraph {
        let mut graph = MarketGraph::new();
        for edge in edges {
            graph.add_edge(edge).expect("valid edge");
        }
        graph
    }

    fn token(byte: u8, symbol: &str) -> Token {
        Token::new(Pubkey::new([byte; 32]), 6, Some(symbol.to_owned()))
    }
}
