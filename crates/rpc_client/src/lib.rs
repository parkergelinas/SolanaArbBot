//! Backward-compatible re-export of the [`ingestion::rpc`] module.
//!
//! All existing consumers that import from `rpc_client` continue to work
//! without modification.  New code should import from `ingestion` directly.

#![forbid(unsafe_code)]

pub use ingestion::{MockRpcClient, RpcClient, RpcClientInterface};

#[cfg(test)]
mod tests {
    use super::{MockRpcClient, RpcClient, RpcClientInterface};
    use ingestion::rpc::MockRpcClient as CanonicalMock;

    #[test]
    fn re_export_is_the_same_type_as_canonical() {
        let _: MockRpcClient = CanonicalMock::new();
    }

    #[test]
    fn mock_rpc_client_subscribes_via_re_export() {
        let client = MockRpcClient::new();

        assert!(client.subscribe_logs().is_ok());
    }

    #[test]
    fn mock_rpc_client_satisfies_interface_via_re_export() {
        fn assert_interface<T: RpcClientInterface>(client: &T) {
            assert!(client.subscribe_logs().is_ok());
        }

        assert_interface(&MockRpcClient::new());
    }
}
