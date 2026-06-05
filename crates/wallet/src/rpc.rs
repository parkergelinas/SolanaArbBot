//! Thin async wrapper around `solana_client`'s nonblocking RPC client.

use std::sync::Arc;

use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;

use crate::error::{WalletError, WalletResult};

/// Identifies the Solana cluster, detected by genesis hash comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Network {
    Mainnet,
    Devnet,
    Localnet,
}

/// Outcome of a transaction simulation.
#[derive(Debug)]
pub struct SimulationResult {
    pub success: bool,
    pub units_consumed: Option<u64>,
    pub logs: Vec<String>,
    pub error: Option<String>,
}

/// Async wrapper around `solana_client::nonblocking::rpc_client::RpcClient`.
pub struct RpcClientWrapper {
    client: solana_client::nonblocking::rpc_client::RpcClient,
    endpoint: String,
    commitment: CommitmentConfig,
}

impl RpcClientWrapper {
    /// Creates a new wrapper.
    ///
    /// `commitment` must be one of `"processed"`, `"confirmed"`, or `"finalized"`.
    pub fn new(endpoint: &str, commitment: &str) -> WalletResult<Self> {
        let commitment_config = parse_commitment(commitment)?;
        let client = solana_client::nonblocking::rpc_client::RpcClient::new_with_commitment(
            endpoint.to_string(),
            commitment_config,
        );
        Ok(Self {
            client,
            endpoint: endpoint.to_string(),
            commitment: commitment_config,
        })
    }

    /// Returns the configured RPC endpoint URL.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Returns the configured commitment level.
    pub fn commitment(&self) -> CommitmentConfig {
        self.commitment
    }

    /// Returns the current SOL balance for `pubkey` in lamports.
    pub async fn get_balance(&self, pubkey: &Pubkey) -> WalletResult<u64> {
        self.client
            .get_balance(pubkey)
            .await
            .map_err(|e| WalletError::Rpc(e.to_string()))
    }

    /// Returns the latest confirmed blockhash.
    pub async fn get_latest_blockhash(&self) -> WalletResult<Hash> {
        self.client
            .get_latest_blockhash()
            .await
            .map_err(|e| WalletError::Rpc(e.to_string()))
    }

    /// Simulates a transaction and returns a structured result.
    pub async fn simulate_transaction(
        &self,
        tx: &Transaction,
    ) -> WalletResult<SimulationResult> {
        let response = self
            .client
            .simulate_transaction(tx)
            .await
            .map_err(|e| WalletError::Rpc(e.to_string()))?;

        let sim = response.value;
        Ok(SimulationResult {
            success: sim.err.is_none(),
            units_consumed: sim.units_consumed,
            logs: sim.logs.unwrap_or_default(),
            error: sim.err.map(|e| e.to_string()),
        })
    }

    /// Fetches the genesis hash and compares it against the well-known hashes for
    /// `expected` network, returning an error on mismatch.
    pub async fn validate_network(&self, expected: Network) -> WalletResult<()> {
        let genesis_hash = self
            .client
            .get_genesis_hash()
            .await
            .map_err(|e| WalletError::Rpc(e.to_string()))?;

        // Well-known genesis hashes (base58).
        const MAINNET_GENESIS: &str = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
        const DEVNET_GENESIS: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";

        let genesis_str = genesis_hash.to_string();
        let detected = if genesis_str == MAINNET_GENESIS {
            Network::Mainnet
        } else if genesis_str == DEVNET_GENESIS {
            Network::Devnet
        } else {
            Network::Localnet
        };

        if detected != expected {
            return Err(WalletError::NetworkMismatch {
                expected: format!("{:?}", expected),
                detected: format!("{:?}", detected),
                endpoint: self.endpoint.clone(),
            });
        }

        tracing::info!(
            "network validated: {:?} at {}",
            expected,
            self.endpoint
        );
        Ok(())
    }
}

fn parse_commitment(s: &str) -> WalletResult<CommitmentConfig> {
    match s {
        "processed" => Ok(CommitmentConfig::processed()),
        "confirmed" => Ok(CommitmentConfig::confirmed()),
        "finalized" => Ok(CommitmentConfig::finalized()),
        other => Err(WalletError::Rpc(format!(
            "unknown commitment level '{}'; expected processed | confirmed | finalized",
            other
        ))),
    }
}

/// Convenience constructor shared by the rest of the crate.
pub fn make_rpc(endpoint: &str, commitment: &str) -> WalletResult<Arc<RpcClientWrapper>> {
    RpcClientWrapper::new(endpoint, commitment).map(Arc::new)
}
