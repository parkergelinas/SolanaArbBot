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
    use stream::StreamIngestor;

    /// Placeholder top-level engine handle.
    #[derive(Clone, Copy, Debug)]
    pub struct MarketAnalysisEngine {
        accounts: AccountCache,
        decoder: DexDecoder,
        execution: ExecutionSimulator,
        graph: MarketGraph,
        pricing: PricingEngine,
        risk: RiskEngine,
        routing: Router,
        rpc_client: RpcClient,
        stream: StreamIngestor,
    }

    impl MarketAnalysisEngine {
        /// Creates a placeholder engine with all workspace boundaries wired.
        #[must_use]
        pub const fn new(
            accounts: AccountCache,
            decoder: DexDecoder,
            execution: ExecutionSimulator,
            graph: MarketGraph,
            pricing: PricingEngine,
            risk: RiskEngine,
            routing: Router,
            rpc_client: RpcClient,
            stream: StreamIngestor,
        ) -> Self {
            Self {
                accounts,
                decoder,
                execution,
                graph,
                pricing,
                risk,
                routing,
                rpc_client,
                stream,
            }
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
            let stream = StreamIngestor::new(rpc_client);

            Self::new(
                AccountCache::new(),
                decoder,
                execution,
                graph,
                pricing,
                RiskEngine::new(),
                routing,
                rpc_client,
                stream,
            )
        }

        /// Performs no-op readiness checks across scaffolded boundaries.
        pub fn ready(&self) -> Result<()> {
            self.accounts.validate()?;
            self.execution.ready()?;
            self.graph.validate()?;
            self.pricing.ready()?;
            self.risk.ready()?;
            self.rpc_client.ready()?;
            self.stream.ready()
        }

        /// Returns the decoder boundary used by the engine.
        #[must_use]
        pub const fn decoder(&self) -> DexDecoder {
            self.decoder
        }

        /// Returns the routing boundary used by the engine.
        #[must_use]
        pub const fn routing(&self) -> Router {
            self.routing
        }
    }
}

pub use runtime::MarketAnalysisEngine;

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
