//! Market graph boundary.
//!
//! This crate will own token-market connectivity used by routing.

#![forbid(unsafe_code)]

pub mod market_graph {
    //! Market graph storage and update responsibilities.

    use std::collections::{HashMap, HashSet};

    use common::{Error, Result, Token};
    use decoder::PoolState;

    const DEFAULT_POOL_FEE_BPS: u64 = 25;
    const MAX_CYCLE_DEPTH: usize = 3;

    type EdgeSignature = ([u8; 32], [u8; 32], u64, u64, u64);

    /// Directed weighted swap edge between two tokens.
    #[derive(Clone, Debug, PartialEq)]
    pub struct Edge {
        pub from: Token,
        pub to: Token,
        pub price: f64,
        pub liquidity: f64,
        pub fee_bps: u64,
    }

    impl Edge {
        /// Creates a directed swap edge.
        #[must_use]
        pub fn new(from: Token, to: Token, price: f64, liquidity: f64, fee_bps: u64) -> Self {
            Self {
                from,
                to,
                price,
                liquidity,
                fee_bps,
            }
        }
    }

    /// Directed token cycle discovered in the market graph.
    #[derive(Clone, Debug, PartialEq)]
    pub struct TokenCycle {
        pub path: Vec<Edge>,
        pub raw_return: f64,
    }

    /// Token relationship graph backed by an adjacency list.
    #[derive(Clone, Debug, Default)]
    pub struct MarketGraph {
        adjacency: HashMap<Token, HashMap<Token, Vec<Edge>>>,
        edge_count: usize,
    }

    impl MarketGraph {
        /// Creates an empty market graph.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Adds one directed edge while preserving existing routes for the pair.
        pub fn add_edge(&mut self, edge: Edge) -> Result<()> {
            validate_edge(&edge)?;

            self.adjacency
                .entry(edge.from.clone())
                .or_default()
                .entry(edge.to.clone())
                .or_default()
                .push(edge);
            self.edge_count += 1;
            Ok(())
        }

        /// Applies a pool state update by replacing its directed token-pair edges.
        ///
        /// Nodes are implicit: adding edges makes each token discoverable through
        /// adjacency traversal. Weights are encoded on each edge as price and
        /// liquidity.
        pub fn apply_pool_state(&mut self, pool: &PoolState) -> Result<()> {
            let liquidity = pool_liquidity(pool)?;
            let (forward_price, reverse_price) = pool_prices(pool)?;
            let forward = Edge::new(
                pool.token_a.clone(),
                pool.token_b.clone(),
                forward_price,
                liquidity,
                DEFAULT_POOL_FEE_BPS,
            );
            let reverse = Edge::new(
                pool.token_b.clone(),
                pool.token_a.clone(),
                reverse_price,
                liquidity,
                DEFAULT_POOL_FEE_BPS,
            );

            self.replace_edges(pool.token_a.clone(), pool.token_b.clone(), vec![forward])?;
            self.replace_edges(pool.token_b.clone(), pool.token_a.clone(), vec![reverse])
        }

        /// Finds all unique directed token cycles up to depth 3.
        ///
        /// Raw return is the product of edge prices only; fees and slippage are
        /// intentionally left to routing/execution layers.
        pub fn find_cycles(&self) -> Vec<TokenCycle> {
            self.find_cycles_up_to_depth(MAX_CYCLE_DEPTH)
                .expect("constant max depth is valid")
        }

        /// Finds all unique directed token cycles up to the requested depth.
        pub fn find_cycles_up_to_depth(&self, max_depth: usize) -> Result<Vec<TokenCycle>> {
            if !(2..=MAX_CYCLE_DEPTH).contains(&max_depth) {
                return Err(Error::InvalidState(
                    "graph cycle depth must be between 2 and 3".to_owned(),
                ));
            }

            let starts = self.source_tokens().cloned().collect::<Vec<_>>();
            let mut search = CycleSearch::new(self, max_depth);

            for start in starts {
                let mut visited = vec![start.clone()];
                let mut path = Vec::with_capacity(max_depth);
                search.search(&start, &start, &mut visited, &mut path);
            }

            let mut cycles = search.into_cycles();
            cycles.sort_by_key(|cycle| canonical_signature(&cycle.path));
            Ok(cycles)
        }

        /// Replaces all edges for one directed token pair.
        pub fn replace_edges(&mut self, from: Token, to: Token, edges: Vec<Edge>) -> Result<()> {
            for edge in &edges {
                validate_edge(edge)?;
                if edge.from != from || edge.to != to {
                    return Err(Error::InvalidState(
                        "replacement edge token pair does not match target pair".to_owned(),
                    ));
                }
            }

            let previous_len = self
                .adjacency
                .get(&from)
                .and_then(|destinations| destinations.get(&to))
                .map_or(0, Vec::len);
            let new_len = edges.len();

            if edges.is_empty() {
                if let Some(destinations) = self.adjacency.get_mut(&from) {
                    destinations.remove(&to);
                    if destinations.is_empty() {
                        self.adjacency.remove(&from);
                    }
                }
            } else {
                self.adjacency.entry(from).or_default().insert(to, edges);
            }

            self.edge_count = self.edge_count + new_len - previous_len;
            Ok(())
        }

        /// Removes all directed edges for one token pair.
        pub fn remove_edges(&mut self, from: &Token, to: &Token) -> Option<Vec<Edge>> {
            let removed = self
                .adjacency
                .get_mut(from)
                .and_then(|destinations| destinations.remove(to));

            if let Some(edges) = &removed {
                self.edge_count -= edges.len();
            }

            if self.adjacency.get(from).is_some_and(HashMap::is_empty) {
                self.adjacency.remove(from);
            }

            removed
        }

        /// Returns all directed edges for a token pair in average O(1).
        #[must_use]
        pub fn edges(&self, from: &Token, to: &Token) -> Option<&[Edge]> {
            self.adjacency
                .get(from)
                .and_then(|destinations| destinations.get(to))
                .map(Vec::as_slice)
        }

        /// Returns true when at least one route exists for a token pair.
        #[must_use]
        pub fn contains_pair(&self, from: &Token, to: &Token) -> bool {
            self.edges(from, to).is_some_and(|edges| !edges.is_empty())
        }

        /// Iterates all outgoing edges for a token.
        pub fn outgoing_edges<'a>(
            &'a self,
            from: &'a Token,
        ) -> impl Iterator<Item = &'a Edge> + 'a {
            self.adjacency
                .get(from)
                .into_iter()
                .flat_map(HashMap::values)
                .flatten()
        }

        /// Iterates directed neighbor tokens for a token.
        pub fn neighbors<'a>(&'a self, from: &'a Token) -> impl Iterator<Item = &'a Token> + 'a {
            self.adjacency.get(from).into_iter().flat_map(HashMap::keys)
        }

        /// Iterates tokens with at least one outgoing edge.
        pub fn source_tokens(&self) -> impl Iterator<Item = &Token> {
            self.adjacency.keys()
        }

        /// Returns the number of nodes that have outgoing edges.
        #[must_use]
        pub fn node_count(&self) -> usize {
            self.adjacency.len()
        }

        /// Returns the number of directed edges.
        #[must_use]
        pub const fn edge_count(&self) -> usize {
            self.edge_count
        }

        /// Validates stored edge invariants.
        pub fn validate(&self) -> Result<()> {
            let counted_edges = self
                .adjacency
                .values()
                .map(|destinations| destinations.values().map(Vec::len).sum::<usize>())
                .sum::<usize>();

            if counted_edges != self.edge_count {
                return Err(Error::InvalidState(
                    "market graph edge count is inconsistent".to_owned(),
                ));
            }

            for (from, destinations) in &self.adjacency {
                for (to, edges) in destinations {
                    for edge in edges {
                        validate_edge(edge)?;
                        if &edge.from != from || &edge.to != to {
                            return Err(Error::InvalidState(
                                "market graph adjacency key mismatch".to_owned(),
                            ));
                        }
                    }
                }
            }

            Ok(())
        }
    }

    struct CycleSearch<'a> {
        graph: &'a MarketGraph,
        max_depth: usize,
        cycles: Vec<TokenCycle>,
        seen: HashSet<Vec<EdgeSignature>>,
    }

    impl<'a> CycleSearch<'a> {
        fn new(graph: &'a MarketGraph, max_depth: usize) -> Self {
            Self {
                graph,
                max_depth,
                cycles: Vec::new(),
                seen: HashSet::new(),
            }
        }

        fn into_cycles(self) -> Vec<TokenCycle> {
            self.cycles
        }

        fn search(
            &mut self,
            start: &Token,
            current: &Token,
            visited: &mut Vec<Token>,
            path: &mut Vec<Edge>,
        ) {
            if path.len() >= self.max_depth {
                return;
            }

            for edge in self.graph.outgoing_edges(current) {
                let next_depth = path.len() + 1;

                if &edge.to == start {
                    if next_depth >= 2 {
                        path.push(edge.clone());
                        let signature = canonical_signature(path);
                        if self.seen.insert(signature) {
                            self.cycles.push(TokenCycle {
                                raw_return: raw_return(path),
                                path: path.clone(),
                            });
                        }
                        path.pop();
                    }
                    continue;
                }

                if next_depth >= self.max_depth || visited.iter().any(|token| token == &edge.to) {
                    continue;
                }

                path.push(edge.clone());
                visited.push(edge.to.clone());
                self.search(start, &edge.to, visited, path);
                visited.pop();
                path.pop();
            }
        }
    }

    fn validate_edge(edge: &Edge) -> Result<()> {
        if !edge.price.is_finite() || edge.price <= 0.0 {
            return Err(Error::InvalidState(
                "edge price must be finite and positive".to_owned(),
            ));
        }

        if !edge.liquidity.is_finite() || edge.liquidity < 0.0 {
            return Err(Error::InvalidState(
                "edge liquidity must be finite and non-negative".to_owned(),
            ));
        }

        Ok(())
    }

    fn pool_liquidity(pool: &PoolState) -> Result<f64> {
        let liquidity = pool.liquidity as f64;
        if !liquidity.is_finite() || liquidity <= 0.0 {
            return Err(Error::InvalidState(
                "pool liquidity must be finite and positive".to_owned(),
            ));
        }

        Ok(liquidity)
    }

    fn pool_prices(pool: &PoolState) -> Result<(f64, f64)> {
        let (forward, reverse) = match pool.reserves {
            Some((reserve_a, reserve_b)) if reserve_a > 0 && reserve_b > 0 => (
                reserve_b as f64 / reserve_a as f64,
                reserve_a as f64 / reserve_b as f64,
            ),
            Some(_) => {
                return Err(Error::InvalidState(
                    "pool reserves must be positive".to_owned(),
                ))
            }
            None => (1.0, 1.0),
        };

        if !forward.is_finite() || !reverse.is_finite() || forward <= 0.0 || reverse <= 0.0 {
            return Err(Error::InvalidState(
                "pool prices must be finite and positive".to_owned(),
            ));
        }

        Ok((forward, reverse))
    }

    fn raw_return(path: &[Edge]) -> f64 {
        path.iter().map(|edge| edge.price).product()
    }

    fn canonical_signature(path: &[Edge]) -> Vec<EdgeSignature> {
        let edge_signatures = path.iter().map(edge_signature).collect::<Vec<_>>();
        let mut best = edge_signatures.clone();

        for offset in 1..edge_signatures.len() {
            let rotated = edge_signatures[offset..]
                .iter()
                .chain(edge_signatures[..offset].iter())
                .copied()
                .collect::<Vec<_>>();
            if rotated < best {
                best = rotated;
            }
        }

        best
    }

    fn edge_signature(edge: &Edge) -> EdgeSignature {
        (
            edge.from.mint().to_bytes(),
            edge.to.mint().to_bytes(),
            edge.price.to_bits(),
            edge.liquidity.to_bits(),
            edge.fee_bps,
        )
    }
}

pub use market_graph::{Edge, MarketGraph, TokenCycle};

#[cfg(test)]
mod tests {
    use super::{Edge, MarketGraph};
    use common::{Pubkey, Token};
    use decoder::{DexType, PoolState};

    #[test]
    fn graph_adds_and_looks_up_directed_edges() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();

        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 150.0, 1_000.0, 25))
            .expect("add edge");

        let edges = graph.edges(&sol, &usdc).expect("pair edges");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].price, 150.0);
        assert!(graph.edges(&usdc, &sol).is_none());
    }

    #[test]
    fn graph_supports_multi_edges_for_same_pair() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();

        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 150.0, 1_000.0, 25))
            .expect("first edge");
        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 149.5, 2_000.0, 30))
            .expect("second edge");

        let edges = graph.edges(&sol, &usdc).expect("pair edges");
        assert_eq!(edges.len(), 2);
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn graph_traverses_outgoing_neighbors() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let usdt = token(3, "USDT");
        let mut graph = MarketGraph::new();

        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 150.0, 1_000.0, 25))
            .expect("sol-usdc");
        graph
            .add_edge(Edge::new(sol.clone(), usdt.clone(), 149.0, 900.0, 25))
            .expect("sol-usdt");

        let outgoing = graph.outgoing_edges(&sol).count();
        let mut neighbors = graph.neighbors(&sol).cloned().collect::<Vec<_>>();
        neighbors.sort_by_key(Token::mint);

        assert_eq!(outgoing, 2);
        assert_eq!(neighbors, vec![usdc, usdt]);
    }

    #[test]
    fn graph_replaces_edges_efficiently() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();

        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 150.0, 1_000.0, 25))
            .expect("old edge");
        graph
            .replace_edges(
                sol.clone(),
                usdc.clone(),
                vec![Edge::new(sol.clone(), usdc.clone(), 151.0, 3_000.0, 20)],
            )
            .expect("replace edge");

        let edges = graph.edges(&sol, &usdc).expect("pair edges");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].price, 151.0);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn graph_rejects_invalid_edge_weights() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();

        let err = graph
            .add_edge(Edge::new(sol, usdc, 0.0, 1_000.0, 25))
            .expect_err("invalid price");

        assert!(matches!(err, common::Error::InvalidState(_)));
    }

    #[test]
    fn graph_builds_edges_from_pool_state_update() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();
        let pool = pool_state(sol.clone(), usdc.clone(), 10_000, Some((1_000, 2_000)));

        graph.apply_pool_state(&pool).expect("apply pool");

        let forward = graph.edges(&sol, &usdc).expect("forward edge");
        let reverse = graph.edges(&usdc, &sol).expect("reverse edge");
        assert_eq!(forward.len(), 1);
        assert_eq!(reverse.len(), 1);
        assert_eq!(forward[0].price, 2.0);
        assert_eq!(reverse[0].price, 0.5);
        assert_eq!(forward[0].liquidity, 10_000.0);
        assert_eq!(graph.edge_count(), 2);
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn graph_replaces_edges_from_repeated_pool_state_update() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();

        graph
            .apply_pool_state(&pool_state(
                sol.clone(),
                usdc.clone(),
                10_000,
                Some((1_000, 2_000)),
            ))
            .expect("initial update");
        graph
            .apply_pool_state(&pool_state(
                sol.clone(),
                usdc.clone(),
                20_000,
                Some((1_000, 2_500)),
            ))
            .expect("replacement update");

        let forward = graph.edges(&sol, &usdc).expect("forward edge");
        assert_eq!(forward.len(), 1);
        assert_eq!(forward[0].price, 2.5);
        assert_eq!(forward[0].liquidity, 20_000.0);
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn graph_rejects_invalid_pool_state_liquidity() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();

        let err = graph
            .apply_pool_state(&pool_state(sol, usdc, 0, Some((1_000, 2_000))))
            .expect_err("invalid liquidity");

        assert!(matches!(err, common::Error::InvalidState(_)));
    }

    #[test]
    fn graph_finds_two_hop_cycle_with_raw_return() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();

        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 2.0, 1_000.0, 25))
            .expect("sol-usdc");
        graph
            .add_edge(Edge::new(usdc, sol, 0.6, 1_000.0, 25))
            .expect("usdc-sol");

        let cycles = graph.find_cycles();

        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].path.len(), 2);
        assert!((cycles[0].raw_return - 1.2).abs() < f64::EPSILON);
    }

    #[test]
    fn graph_finds_three_hop_cycle() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let usdt = token(3, "USDT");
        let mut graph = MarketGraph::new();

        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 2.0, 1_000.0, 25))
            .expect("sol-usdc");
        graph
            .add_edge(Edge::new(usdc.clone(), usdt.clone(), 1.1, 1_000.0, 25))
            .expect("usdc-usdt");
        graph
            .add_edge(Edge::new(usdt, sol, 0.55, 1_000.0, 25))
            .expect("usdt-sol");

        let cycles = graph.find_cycles();

        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].path.len(), 3);
        assert!((cycles[0].raw_return - 1.21).abs() < 0.0000000001);
    }

    #[test]
    fn graph_keeps_distinct_multi_edge_cycles() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();

        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 2.0, 1_000.0, 25))
            .expect("first sol-usdc");
        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 2.1, 1_000.0, 25))
            .expect("second sol-usdc");
        graph
            .add_edge(Edge::new(usdc, sol, 0.5, 1_000.0, 25))
            .expect("usdc-sol");

        let cycles = graph.find_cycles();

        assert_eq!(cycles.len(), 2);
        assert!(cycles
            .iter()
            .any(|cycle| (cycle.raw_return - 1.0).abs() < f64::EPSILON));
        assert!(cycles
            .iter()
            .any(|cycle| (cycle.raw_return - 1.05).abs() < 0.0000000001));
    }

    #[test]
    fn graph_deduplicates_cycle_rotations() {
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let mut graph = MarketGraph::new();

        graph
            .add_edge(Edge::new(sol.clone(), usdc.clone(), 2.0, 1_000.0, 25))
            .expect("sol-usdc");
        graph
            .add_edge(Edge::new(usdc, sol, 0.5, 1_000.0, 25))
            .expect("usdc-sol");

        assert_eq!(graph.find_cycles().len(), 1);
    }

    #[test]
    fn graph_rejects_cycle_depth_above_three() {
        let graph = MarketGraph::new();

        let err = graph
            .find_cycles_up_to_depth(4)
            .expect_err("depth too high");

        assert!(matches!(err, common::Error::InvalidState(_)));
    }

    fn token(byte: u8, symbol: &str) -> Token {
        Token::new(Pubkey::new([byte; 32]), 6, Some(symbol.to_owned()))
    }

    fn pool_state(
        token_a: Token,
        token_b: Token,
        liquidity: u128,
        reserves: Option<(u64, u64)>,
    ) -> PoolState {
        PoolState {
            dex: DexType::Raydium,
            token_a,
            token_b,
            liquidity,
            reserves,
        }
    }
}
