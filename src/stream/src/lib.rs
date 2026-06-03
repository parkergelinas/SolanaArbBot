//! Real-time Solana ingestion boundary.
//!
//! This crate will own subscription streams and hand raw updates to decoders.

#![forbid(unsafe_code)]

pub mod ingestion {
    //! Stream setup, lifecycle, and update handoff responsibilities.

    use common::Result;
    use rpc_client::RpcClient;

    /// Placeholder stream ingestion handle.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct StreamIngestor {
        rpc_client: RpcClient,
    }

    impl StreamIngestor {
        /// Creates a placeholder stream ingestor.
        #[must_use]
        pub const fn new(rpc_client: RpcClient) -> Self {
            Self { rpc_client }
        }

        /// Returns the RPC boundary used by ingestion.
        #[must_use]
        pub const fn rpc_client(&self) -> RpcClient {
            self.rpc_client
        }

        /// Performs a no-op readiness check for the scaffold.
        pub const fn ready(&self) -> Result<()> {
            self.rpc_client.ready()
        }
    }
}

pub use ingestion::StreamIngestor;

#[cfg(test)]
mod tests {
    use super::StreamIngestor;
    use rpc_client::RpcClient;

    #[test]
    fn stream_ingestor_placeholder_is_ready() {
        let ingestor = StreamIngestor::new(RpcClient::new());

        assert!(ingestor.ready().is_ok());
        assert!(ingestor.rpc_client().ready().is_ok());
    }
}
