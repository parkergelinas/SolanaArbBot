//! Minimal async JSON-RPC 2.0 client for the Solana network.
//!
//! Uses `reqwest 0.11` (hyper 0.14 / h2 0.3) directly instead of `solana-client`
//! to avoid pulling in that crate's edition2024-incompatible dependency tree.

use std::str::FromStr;
use std::sync::Arc;

use common::Pubkey;
use serde_json::{json, Value};

use crate::error::{WalletError, WalletResult};
use crate::types::Blockhash;

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

/// Async JSON-RPC 2.0 wrapper for the Solana RPC API.
pub struct RpcClientWrapper {
    client: reqwest::Client,
    endpoint: String,
    commitment: String,
}

impl RpcClientWrapper {
    /// Creates a new wrapper.
    ///
    /// `commitment` must be `"processed"`, `"confirmed"`, or `"finalized"`.
    pub fn new(endpoint: &str, commitment: &str) -> WalletResult<Self> {
        validate_commitment(commitment)?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| WalletError::Rpc(e.to_string()))?;
        Ok(Self {
            client,
            endpoint: endpoint.to_string(),
            commitment: commitment.to_string(),
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn commitment(&self) -> &str {
        &self.commitment
    }

    /// Returns the current SOL balance for `pubkey` in lamports.
    pub async fn get_balance(&self, pubkey: &Pubkey) -> WalletResult<u64> {
        let pubkey_b58 = bs58::encode(pubkey.as_bytes()).into_string();
        let result = self
            .call(
                "getBalance",
                json!([pubkey_b58, {"commitment": self.commitment}]),
            )
            .await?;
        result["value"]
            .as_u64()
            .ok_or_else(|| WalletError::Rpc("getBalance: missing or invalid 'value'".into()))
    }

    /// Returns the latest confirmed blockhash.
    pub async fn get_latest_blockhash(&self) -> WalletResult<Blockhash> {
        let result = self
            .call(
                "getLatestBlockhash",
                json!([{"commitment": self.commitment}]),
            )
            .await?;
        let hash_str = result["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| WalletError::Rpc("getLatestBlockhash: missing blockhash".into()))?;
        Blockhash::from_str(hash_str)
    }

    /// Simulates a pre-serialised, base64-encoded transaction.
    ///
    /// The caller is responsible for serialising the transaction into bytes and
    /// base64-encoding them before passing to this method.
    pub async fn simulate_transaction_bytes(
        &self,
        tx_base64: &str,
    ) -> WalletResult<SimulationResult> {
        let result = self
            .call(
                "simulateTransaction",
                json!([
                    tx_base64,
                    {"encoding": "base64", "commitment": self.commitment}
                ]),
            )
            .await?;

        let value = &result["value"];
        let error = if value["err"].is_null() {
            None
        } else {
            Some(value["err"].to_string())
        };
        let logs = value["logs"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let units_consumed = value["unitsConsumed"].as_u64();

        Ok(SimulationResult {
            success: error.is_none(),
            units_consumed,
            logs,
            error,
        })
    }

    /// Fetches the genesis hash and validates that the cluster matches `expected`.
    pub async fn validate_network(&self, expected: Network) -> WalletResult<()> {
        let result = self.call("getGenesisHash", json!([])).await?;
        let genesis_str = result
            .as_str()
            .ok_or_else(|| WalletError::Rpc("getGenesisHash: expected string result".into()))?;

        const MAINNET_GENESIS: &str = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
        const DEVNET_GENESIS: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";

        let detected = if genesis_str == MAINNET_GENESIS {
            Network::Mainnet
        } else if genesis_str == DEVNET_GENESIS {
            Network::Devnet
        } else {
            Network::Localnet
        };

        if detected != expected {
            return Err(WalletError::NetworkMismatch {
                expected: format!("{expected:?}"),
                detected: format!("{detected:?}"),
                endpoint: self.endpoint.clone(),
            });
        }

        tracing::info!("network validated: {:?} at {}", expected, self.endpoint);
        Ok(())
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    async fn call(&self, method: &str, params: Value) -> WalletResult<Value> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| WalletError::Rpc(e.to_string()))?;

        let json: Value = resp
            .json()
            .await
            .map_err(|e| WalletError::Rpc(e.to_string()))?;

        if let Some(err) = json.get("error") {
            return Err(WalletError::Rpc(format!("RPC error: {err}")));
        }

        json.get("result")
            .cloned()
            .ok_or_else(|| WalletError::Rpc("missing 'result' field in RPC response".into()))
    }
}

fn validate_commitment(s: &str) -> WalletResult<()> {
    match s {
        "processed" | "confirmed" | "finalized" => Ok(()),
        other => Err(WalletError::Rpc(format!(
            "unknown commitment '{other}'; expected processed | confirmed | finalized"
        ))),
    }
}

/// Convenience constructor that wraps the result in `Arc`.
pub fn make_rpc(endpoint: &str, commitment: &str) -> WalletResult<Arc<RpcClientWrapper>> {
    RpcClientWrapper::new(endpoint, commitment).map(Arc::new)
}
