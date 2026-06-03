//! System orchestration boundary.
//!
//! This crate will compose ingestion, decoding, analysis, routing, execution,
//! and risk components into the market analysis engine.

#![forbid(unsafe_code)]

pub mod runtime {
    //! Engine component wiring and lifecycle responsibilities.

    use accounts::AccountCache;
    use common::Result;
    use decoder::DexDecoder;
    use execution::ExecutionSimulator;
    use graph::MarketGraph;
    use pricing::PricingEngine;
    use risk::RiskEngine;
    use routing::Router;
    use rpc_client::RpcClient;
    use stream::{StreamIngestor, StreamReceiver};

    /// Placeholder bundle for engine component boundaries.
    #[derive(Debug)]
    pub struct MarketAnalysisComponents {
        pub accounts: AccountCache,
        pub decoder: DexDecoder,
        pub execution: ExecutionSimulator,
        pub graph: MarketGraph,
        pub pricing: PricingEngine,
        pub risk: RiskEngine,
        pub routing: Router,
        pub rpc_client: RpcClient,
        pub stream: StreamIngestor,
        pub stream_receiver: StreamReceiver,
    }

    /// Placeholder top-level engine handle.
    #[derive(Debug)]
    pub struct MarketAnalysisEngine {
        components: MarketAnalysisComponents,
    }

    impl MarketAnalysisEngine {
        /// Creates a placeholder engine from already-wired component boundaries.
        #[must_use]
        pub const fn new(components: MarketAnalysisComponents) -> Self {
            Self { components }
        }

        /// Creates a scaffold engine using placeholder components.
        #[must_use]
        pub fn scaffold() -> Self {
            let decoder = DexDecoder::new();
            let graph = MarketGraph::new();
            let pricing = PricingEngine::new(decoder);
            let routing = Router::new(graph, pricing);
            let execution = ExecutionSimulator::new(routing);
            let rpc_client = RpcClient::new();
            let (stream, stream_receiver) = StreamIngestor::new(rpc_client);

            Self::new(MarketAnalysisComponents {
                accounts: AccountCache::new(),
                decoder,
                execution,
                graph,
                pricing,
                risk: RiskEngine::new(),
                routing,
                rpc_client,
                stream,
                stream_receiver,
            })
        }

        /// Performs no-op readiness checks across scaffolded boundaries.
        pub fn ready(&self) -> Result<()> {
            self.components.accounts.validate()?;
            self.components.execution.ready()?;
            self.components.graph.validate()?;
            self.components.pricing.ready()?;
            self.components.risk.ready()?;
            self.components.rpc_client.ready()?;
            self.components.stream.ready()
        }

        /// Returns the decoder boundary used by the engine.
        #[must_use]
        pub const fn decoder(&self) -> DexDecoder {
            self.components.decoder
        }

        /// Returns the routing boundary used by the engine.
        #[must_use]
        pub const fn routing(&self) -> Router {
            self.components.routing
        }
    }
}

pub use runtime::{MarketAnalysisComponents, MarketAnalysisEngine};

#[cfg(test)]
mod tests {
    use super::MarketAnalysisEngine;

    #[test]
    fn scaffold_engine_wires_placeholder_components() {
        let engine = MarketAnalysisEngine::scaffold();

        assert!(engine.ready().is_ok());
        assert!(engine.decoder().decode(&[]).is_none());
        assert!(engine.routing().graph().validate().is_ok());
    }
}
