//! Solana RPC client boundary.
//!
//! This crate will own RPC transport configuration and request orchestration.

#![forbid(unsafe_code)]

pub mod client {
    //! RPC client construction and lifecycle responsibilities.

    use common::Result;

    /// Placeholder RPC client handle.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct RpcClient;

    impl RpcClient {
        /// Creates a placeholder RPC client.
        #[must_use]
        pub const fn new() -> Self {
            Self
        }

        /// Performs a no-op readiness check for the scaffold.
        pub const fn ready(&self) -> Result<()> {
            Ok(())
        }
    }
}

pub use client::RpcClient;

#[cfg(test)]
mod tests {
    use super::RpcClient;

    #[test]
    fn rpc_client_placeholder_is_ready() {
        assert!(RpcClient::new().ready().is_ok());
    }
}
