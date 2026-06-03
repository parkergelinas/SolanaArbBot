//! System orchestration boundary.
//!
//! This crate will compose ingestion, decoding, analysis, routing, execution,
//! and risk components into the market analysis engine.

#![forbid(unsafe_code)]

pub mod runtime {
    //! Engine component wiring and lifecycle responsibilities.

    use std::time::Duration;

    use accounts::AccountCache;
    use common::{Error, MarketEvent, Pubkey, Result};
    use decoder::{raydium, DexDecoder, DexType, PoolState};
    use execution::ExecutionSimulator;
    use graph::{Edge, MarketGraph};
    use pricing::PricingEngine;
    use risk::RiskEngine;
    use routing::Router;
    use rpc_client::{MockRpcClient, RpcClient};
    use stream::{EventBus, EventSubscriber, IngestionConfig, IngestionEngine};
    use tracing::{debug, trace};

    const RAYDIUM_TOKEN_A_OFFSET: usize = 0;
    const RAYDIUM_TOKEN_B_OFFSET: usize = 32;
    const RAYDIUM_TOKEN_A_DECIMALS_OFFSET: usize = 64;
    const RAYDIUM_TOKEN_B_DECIMALS_OFFSET: usize = 65;
    const RAYDIUM_LIQUIDITY_OFFSET: usize = 66;
    const RAYDIUM_RESERVE_A_OFFSET: usize = 82;
    const RAYDIUM_RESERVE_B_OFFSET: usize = 90;
    const DEFAULT_EVENT_TIMEOUT_MS: u64 = 100;

    /// Bundle for engine component boundaries.
    #[derive(Debug)]
    pub struct MarketAnalysisComponents {
        pub accounts: AccountCache,
        pub decoder: DexDecoder,
        pub execution: ExecutionSimulator,
        pub graph: MarketGraph,
        pub pricing: PricingEngine,
        pub risk: RiskEngine,
        pub routing: Router,
        pub rpc_client: MockRpcClient,
        pub event_bus: EventBus,
        pub ingestion: IngestionEngine,
    }

    /// Runtime engine configuration.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct EngineConfig {
        max_events: usize,
        event_timeout: Duration,
        initial_amount: f64,
    }

    impl EngineConfig {
        /// Creates engine runtime configuration.
        pub fn new(
            max_events: usize,
            event_timeout: Duration,
            initial_amount: f64,
        ) -> Result<Self> {
            if max_events == 0 {
                return Err(Error::InvalidState(
                    "engine max events must be greater than zero".to_owned(),
                ));
            }
            if event_timeout.is_zero() {
                return Err(Error::InvalidState(
                    "engine event timeout must be greater than zero".to_owned(),
                ));
            }
            if !initial_amount.is_finite() || initial_amount <= 0.0 {
                return Err(Error::InvalidState(
                    "engine initial amount must be finite and positive".to_owned(),
                ));
            }

            Ok(Self {
                max_events,
                event_timeout,
                initial_amount,
            })
        }

        /// Returns maximum events processed before a loop exits.
        #[must_use]
        pub const fn max_events(self) -> usize {
            self.max_events
        }

        /// Returns subscriber receive timeout.
        #[must_use]
        pub const fn event_timeout(self) -> Duration {
            self.event_timeout
        }

        /// Returns initial notional used for route simulation.
        #[must_use]
        pub const fn initial_amount(self) -> f64 {
            self.initial_amount
        }
    }

    impl Default for EngineConfig {
        fn default() -> Self {
            Self {
                max_events: 16,
                event_timeout: Duration::from_millis(DEFAULT_EVENT_TIMEOUT_MS),
                initial_amount: 10.0,
            }
        }
    }

    /// Runtime pipeline counters.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct PipelineReport {
        pub stream_events: usize,
        pub decoded_pools: usize,
        pub graph_edges: usize,
        pub routes_found: usize,
        pub simulated_routes: usize,
        pub allowed_routes: usize,
        pub rejected_routes: usize,
    }

    struct RawPoolUpdate {
        dex: DexType,
        data: Vec<u8>,
    }

    /// Top-level runtime engine handle.
    #[derive(Debug)]
    pub struct Engine {
        components: MarketAnalysisComponents,
        config: EngineConfig,
    }

    impl Engine {
        /// Creates an engine from already-wired component boundaries.
        #[must_use]
        pub fn new(components: MarketAnalysisComponents) -> Self {
            Self::with_config(components, EngineConfig::default())
        }

        /// Creates an engine from components and explicit runtime configuration.
        #[must_use]
        pub const fn with_config(
            components: MarketAnalysisComponents,
            config: EngineConfig,
        ) -> Self {
            Self { components, config }
        }

        /// Creates a scaffold engine using placeholder components.
        #[must_use]
        pub fn scaffold() -> Self {
            Self::scaffold_with_config(EngineConfig::default())
        }

        /// Creates a scaffold engine with explicit runtime configuration.
        #[must_use]
        pub fn scaffold_with_config(config: EngineConfig) -> Self {
            let decoder = DexDecoder::new();
            let graph = MarketGraph::new();
            let pricing = PricingEngine::new(decoder);
            let routing = Router::new(graph.clone(), pricing);
            let execution = ExecutionSimulator::new(routing.clone());
            let rpc_client = MockRpcClient::new();
            let event_bus = EventBus::new();
            let ingestion = IngestionEngine::new(event_bus.clone(), IngestionConfig::default());

            let components = MarketAnalysisComponents {
                accounts: AccountCache::new(),
                decoder,
                execution,
                graph,
                pricing,
                risk: RiskEngine::new(),
                routing,
                rpc_client,
                event_bus,
                ingestion,
            };

            Self::with_config(components, config)
        }

        /// Runs the real-time pipeline loop.
        pub async fn run_loop(&mut self) -> Result<PipelineReport> {
            self.ready()?;
            self.components.rpc_client.subscribe_logs()?;

            let mut report = PipelineReport::default();
            let mut subscriber = self.components.event_bus.subscribe();
            let ingestion = self.components.ingestion.clone().start();

            debug!(
                max_events = self.config.max_events(),
                initial_amount = self.config.initial_amount(),
                "starting engine run loop"
            );

            while report.stream_events < self.config.max_events() {
                let (next_subscriber, event) =
                    recv_stream_event(subscriber, self.config.event_timeout()).await?;
                subscriber = next_subscriber;

                let Some(event) = event else {
                    if ingestion.is_finished() {
                        break;
                    }
                    continue;
                };

                self.process_stream_event(event, &mut report)?;
            }

            if !ingestion.is_finished() {
                ingestion.abort();
            } else {
                ingestion.await.map_err(|err| {
                    Error::InternalError(format!("engine ingestion task failed: {err}"))
                })??;
            }

            debug!(?report, "engine run loop finished");
            Ok(report)
        }

        fn process_stream_event(
            &mut self,
            event: MarketEvent,
            report: &mut PipelineReport,
        ) -> Result<()> {
            trace!(?event, "engine received stream event");
            report.stream_events += 1;

            let raw = placeholder_raw_update(report.stream_events);
            let pool = self.components.decoder.decode(raw.dex, &raw.data)?;
            report.decoded_pools += 1;

            self.components.pricing.ready()?;
            update_graph_from_pool(&mut self.components.graph, &pool)?;
            report.graph_edges = self.components.graph.edge_count();

            self.components.routing =
                Router::new(self.components.graph.clone(), self.components.pricing);
            self.components.execution = ExecutionSimulator::new(self.components.routing.clone());

            let routes = self.components.routing.find_arbitrage_cycles();
            report.routes_found += routes.len();

            for route in routes {
                match self
                    .components
                    .execution
                    .simulate_route(&route, self.config.initial_amount())
                {
                    Ok(execution) => {
                        report.simulated_routes += 1;
                        let decision = self.components.risk.evaluate_route(&route, &execution);
                        if decision.allowed {
                            report.allowed_routes += 1;
                        } else {
                            report.rejected_routes += 1;
                        }
                    }
                    Err(_) => report.rejected_routes += 1,
                }
            }

            Ok(())
        }

        /// Performs readiness checks across scaffolded boundaries.
        pub fn ready(&self) -> Result<()> {
            self.components.accounts.validate()?;
            self.components.execution.ready()?;
            self.components.graph.validate()?;
            self.components.pricing.ready()?;
            self.components.risk.ready()?;
            self.components.rpc_client.ready()
        }

        /// Returns the decoder boundary used by the engine.
        #[must_use]
        pub const fn decoder(&self) -> DexDecoder {
            self.components.decoder
        }

        /// Returns the routing boundary used by the engine.
        #[must_use]
        pub const fn routing(&self) -> &Router {
            &self.components.routing
        }

        /// Returns the graph boundary used by the engine.
        #[must_use]
        pub const fn graph(&self) -> &MarketGraph {
            &self.components.graph
        }

        /// Returns runtime configuration.
        #[must_use]
        pub const fn config(&self) -> EngineConfig {
            self.config
        }
    }

    async fn recv_stream_event(
        subscriber: EventSubscriber,
        timeout: Duration,
    ) -> Result<(EventSubscriber, Option<MarketEvent>)> {
        tokio::task::spawn_blocking(move || {
            let event = subscriber.recv_timeout(timeout);
            (subscriber, event)
        })
        .await
        .map_err(|err| Error::InternalError(format!("engine stream receive failed: {err}")))
        .and_then(|(subscriber, event)| event.map(|event| (subscriber, event)))
    }

    fn update_graph_from_pool(graph: &mut MarketGraph, pool: &PoolState) -> Result<()> {
        let liquidity = pool.liquidity as f64;
        let (forward_price, reverse_price) = match pool.reserves {
            Some((reserve_a, reserve_b)) if reserve_a > 0 && reserve_b > 0 => (
                reserve_b as f64 / reserve_a as f64,
                reserve_a as f64 / reserve_b as f64,
            ),
            _ => (1.0, 1.0),
        };

        let forward = Edge::new(
            pool.token_a.clone(),
            pool.token_b.clone(),
            forward_price,
            liquidity,
            25,
        );
        let reverse = Edge::new(
            pool.token_b.clone(),
            pool.token_a.clone(),
            reverse_price,
            liquidity,
            25,
        );

        graph.replace_edges(pool.token_a.clone(), pool.token_b.clone(), vec![forward])?;
        graph.replace_edges(pool.token_b.clone(), pool.token_a.clone(), vec![reverse])
    }

    fn placeholder_raw_update(sequence: usize) -> RawPoolUpdate {
        let _ = sequence;
        RawPoolUpdate {
            dex: DexType::Raydium,
            data: raydium_placeholder_pool(),
        }
    }

    fn raydium_placeholder_pool() -> Vec<u8> {
        let mut data = vec![0; raydium::RAYDIUM_POOL_DATA_LEN];
        write_pubkey(&mut data, RAYDIUM_TOKEN_A_OFFSET, Pubkey::new([1; 32]));
        write_pubkey(&mut data, RAYDIUM_TOKEN_B_OFFSET, Pubkey::new([2; 32]));
        data[RAYDIUM_TOKEN_A_DECIMALS_OFFSET] = 6;
        data[RAYDIUM_TOKEN_B_DECIMALS_OFFSET] = 6;
        write_u128(&mut data, RAYDIUM_LIQUIDITY_OFFSET, 100_000);
        write_u64(&mut data, RAYDIUM_RESERVE_A_OFFSET, 1_000);
        write_u64(&mut data, RAYDIUM_RESERVE_B_OFFSET, 2_000);
        data
    }

    fn write_pubkey(data: &mut [u8], offset: usize, value: Pubkey) {
        data[offset..offset + 32].copy_from_slice(value.as_bytes());
    }

    fn write_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u128(data: &mut [u8], offset: usize, value: u128) {
        data[offset..offset + 16].copy_from_slice(&value.to_le_bytes());
    }
}

pub use runtime::{Engine, EngineConfig, MarketAnalysisComponents, PipelineReport};

/// Backwards-compatible name for the system runtime.
pub type MarketAnalysisEngine = Engine;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Engine, EngineConfig, MarketAnalysisEngine};

    #[test]
    fn scaffold_engine_wires_placeholder_components() {
        let engine = MarketAnalysisEngine::scaffold();

        assert!(engine.ready().is_ok());
        assert!(engine.routing().graph().validate().is_ok());
    }

    #[tokio::test]
    async fn engine_run_loop_processes_pipeline_events() {
        let config = EngineConfig::new(4, Duration::from_millis(100), 10.0).expect("config");
        let mut engine = Engine::scaffold_with_config(config);

        let report = engine.run_loop().await.expect("run loop");

        assert_eq!(report.stream_events, 4);
        assert_eq!(report.decoded_pools, 4);
        assert!(report.graph_edges >= 2);
        assert!(report.routes_found > 0);
        assert!(report.simulated_routes > 0);
        assert!(engine.graph().validate().is_ok());
    }
}
