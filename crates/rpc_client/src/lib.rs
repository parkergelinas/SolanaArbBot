//! Solana RPC client boundary.
//!
//! This crate owns the transport-agnostic RPC interface used by ingestion and
//! execution components. Concrete Solana JSON-RPC or Yellowstone gRPC clients
//! can implement the same traits without changing downstream crate APIs.

#![forbid(unsafe_code)]

pub mod client {
    //! Transport-agnostic RPC traits and development implementations.

    use common::{Error, Pubkey, Result};
    use tracing::trace;

    /// Abstract Solana RPC surface used by the workspace.
    pub trait RpcClient {
        /// Fetches raw account data for a public key.
        fn get_account(&self, pubkey: Pubkey) -> Result<Vec<u8>>;

        /// Subscribes to logs with the concrete transport implementation.
        fn subscribe_logs(&self) -> Result<()>;
    }

    /// Future-proof alias trait for swappable RPC transports such as Yellowstone gRPC.
    pub trait RpcClientInterface: RpcClient {}

    impl<T> RpcClientInterface for T where T: RpcClient + ?Sized {}

    /// Deterministic development RPC client used by tests and local pipelines.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MockRpcClient {
        account_data_len: usize,
        logs_subscription_enabled: bool,
    }

    impl MockRpcClient {
        /// Default fake account data length.
        pub const DEFAULT_ACCOUNT_DATA_LEN: usize = 128;

        /// Creates a mock RPC client with deterministic account data.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                account_data_len: Self::DEFAULT_ACCOUNT_DATA_LEN,
                logs_subscription_enabled: true,
            }
        }

        /// Creates a mock RPC client with explicit account data length.
        pub fn with_account_data_len(account_data_len: usize) -> Result<Self> {
            if account_data_len == 0 {
                return Err(Error::InvalidState(
                    "mock account data length must be greater than zero".to_owned(),
                ));
            }

            Ok(Self {
                account_data_len,
                logs_subscription_enabled: true,
            })
        }

        /// Creates a mock RPC client that returns an error on log subscription.
        #[must_use]
        pub const fn with_logs_subscription_disabled() -> Self {
            Self {
                account_data_len: Self::DEFAULT_ACCOUNT_DATA_LEN,
                logs_subscription_enabled: false,
            }
        }

        /// Performs a readiness check for scaffold wiring.
        pub const fn ready(&self) -> Result<()> {
            Ok(())
        }

        /// Returns fake account data length.
        #[must_use]
        pub const fn account_data_len(&self) -> usize {
            self.account_data_len
        }
    }

    impl Default for MockRpcClient {
        fn default() -> Self {
            Self::new()
        }
    }

    impl RpcClient for MockRpcClient {
        fn get_account(&self, pubkey: Pubkey) -> Result<Vec<u8>> {
            trace!(?pubkey, len = self.account_data_len, "mock get_account");
            Ok(fake_account_data(pubkey, self.account_data_len))
        }

        fn subscribe_logs(&self) -> Result<()> {
            if self.logs_subscription_enabled {
                trace!("mock subscribe_logs");
                Ok(())
            } else {
                Err(Error::RpcError(
                    "mock log subscriptions are disabled".to_owned(),
                ))
            }
        }
    }

    fn fake_account_data(pubkey: Pubkey, len: usize) -> Vec<u8> {
        let key = pubkey.to_bytes();
        let mut data = Vec::with_capacity(len);

        for index in 0..len {
            data.push(key[index % key.len()].wrapping_add(index as u8));
        }

        data
    }
}

pub use client::{MockRpcClient, RpcClient, RpcClientInterface};

#[cfg(test)]
mod tests {
    use super::{MockRpcClient, RpcClient, RpcClientInterface};
    use common::{Error, Pubkey};

    #[test]
    fn mock_rpc_client_returns_deterministic_account_data() {
        let client = MockRpcClient::with_account_data_len(8).expect("client");
        let pubkey = Pubkey::new([3; 32]);

        let first = client.get_account(pubkey).expect("first account");
        let second = client.get_account(pubkey).expect("second account");

        assert_eq!(first, second);
        assert_eq!(first, vec![3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn mock_rpc_client_subscribes_to_logs() {
        let client = MockRpcClient::new();

        assert!(client.ready().is_ok());
        assert!(client.subscribe_logs().is_ok());
    }

    #[test]
    fn mock_rpc_client_can_fail_log_subscription() {
        let client = MockRpcClient::with_logs_subscription_disabled();
        let err = client.subscribe_logs().expect_err("disabled logs");

        assert!(matches!(err, Error::RpcError(_)));
    }

    #[test]
    fn mock_rpc_client_rejects_zero_account_data_len() {
        let err = MockRpcClient::with_account_data_len(0).expect_err("invalid len");

        assert!(matches!(err, Error::InvalidState(_)));
    }

    #[test]
    fn mock_rpc_client_satisfies_interface_trait() {
        fn assert_interface<T: RpcClientInterface>(client: &T) {
            assert!(client.subscribe_logs().is_ok());
        }

        assert_interface(&MockRpcClient::new());
    }
}
