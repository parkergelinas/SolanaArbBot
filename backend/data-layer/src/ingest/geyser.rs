//! Yellowstone gRPC Geyser client — production streaming account consumer.
//!
//! # Environment variables
//!
//! | Variable              | Description                                      |
//! |-----------------------|--------------------------------------------------|
//! | `GEYSER_GRPC_ENDPOINT`| gRPC endpoint, e.g. `https://ams1.rpc.exoscale.ch:10000` |
//! | `GEYSER_GRPC_TOKEN`   | Bearer token for authentication                  |
//!
//! # Reconnection
//!
//! On disconnect the client retries with exponential back-off starting at 500 ms,
//! doubling on each attempt, for up to 16 attempts total.
//!
//! # Feature gate
//!
//! Full gRPC support is compiled only with `--features yellowstone`.  Without it
//! the adapter logs a warning and keeps the pipeline alive with stub events.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::Sender;
use tracing::{info, warn};

use super::adapter::IngestionAdapter;
use crate::types::{PoolPrice, RawUpdate};

const MAX_RECONNECT_ATTEMPTS: u32 = 16;
const INITIAL_BACKOFF_MS: u64 = 500;

/// Broadcast channel capacity for pool price events.
pub const POOL_PRICE_CHANNEL_CAPACITY: usize = 1024;

/// Shared broadcast sender for [`PoolPrice`] events from the Geyser stream.
///
/// Obtain the matching receiver with [`tokio::sync::broadcast::Sender::subscribe`].
pub type PoolPriceSender = tokio::sync::broadcast::Sender<PoolPrice>;
pub type PoolPriceReceiver = tokio::sync::broadcast::Receiver<PoolPrice>;

/// Creates a new pool-price broadcast channel pair.
pub fn pool_price_channel() -> (PoolPriceSender, PoolPriceReceiver) {
    tokio::sync::broadcast::channel(POOL_PRICE_CHANNEL_CAPACITY)
}

/// Yellowstone gRPC Geyser adapter.
///
/// Constructed from environment variables; `from_env()` returns `None` when
/// `GEYSER_GRPC_ENDPOINT` is not set.
pub struct GeyserAdapter {
    pub endpoint: String,
    pub token: Option<String>,
    /// Pool accounts to subscribe to (base-58 encoded pubkeys).
    pub pool_addresses: Vec<String>,
}

impl GeyserAdapter {
    pub fn from_env() -> Option<Self> {
        let endpoint = std::env::var("GEYSER_GRPC_ENDPOINT")
            .or_else(|_| std::env::var("YELLOWSTONE_ENDPOINT"))
            .ok()
            .filter(|s| !s.is_empty())?;

        let token = std::env::var("GEYSER_GRPC_TOKEN").ok();
        Some(Self {
            endpoint,
            token,
            pool_addresses: vec![],
        })
    }

    /// Attach a list of pool accounts to subscribe to.
    pub fn with_pools(mut self, pools: Vec<String>) -> Self {
        self.pool_addresses = pools;
        self
    }
}

impl IngestionAdapter for GeyserAdapter {
    fn name(&self) -> &'static str {
        "yellowstone-grpc"
    }

    fn spawn(self: Box<Self>, tx: Sender<RawUpdate>) -> anyhow::Result<()> {
        let endpoint = self.endpoint.clone();
        let token = self.token.clone();
        let pools = self.pool_addresses.clone();

        #[cfg(feature = "yellowstone")]
        {
            spawn_yellowstone_grpc(endpoint, token, pools, tx);
            return Ok(());
        }

        #[cfg(not(feature = "yellowstone"))]
        {
            warn!(
                endpoint = %endpoint,
                "GEYSER_GRPC_ENDPOINT set but crate built without `yellowstone` feature — \
                 rebuild with: cargo build -p data-layer --features yellowstone"
            );
            std::thread::spawn(move || {
                let _ = (token, pools); // suppress unused warnings
                loop {
                    if tx
                        .send(RawUpdate {
                            slot: 0,
                            signature: "geyser_stub".into(),
                            logs: vec!["Program log: awaiting yellowstone feature".into()],
                            accounts: vec![],
                            timestamp: now_ms(),
                        })
                        .is_err()
                    {
                        break;
                    }
                    std::thread::sleep(Duration::from_secs(30));
                }
            });
            Ok(())
        }
    }
}

#[cfg(feature = "yellowstone")]
fn spawn_yellowstone_grpc(
    endpoint: String,
    token: Option<String>,
    pool_addresses: Vec<String>,
    raw_tx: Sender<RawUpdate>,
) {
    use futures_util::StreamExt;
    use std::collections::HashMap;
    use yellowstone_grpc_client::GeyserGrpcClient;
    use yellowstone_grpc_proto::geyser::{
        subscribe_request_filter_accounts_filter::Filter as AccountFilter,
        subscribe_request_filter_accounts_filter_memcmp::Data as MemcmpData,
        CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
        SubscribeRequestFilterAccountsFilter, SubscribeRequestFilterAccountsFilterMemcmp,
    };

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime for geyser");

        rt.block_on(async move {
            let mut attempt = 0u32;
            let mut backoff_ms = INITIAL_BACKOFF_MS;

            loop {
                info!(
                    endpoint = %endpoint,
                    attempt,
                    "yellowstone-grpc connecting"
                );

                let result = connect_and_stream(
                    &endpoint,
                    token.as_deref(),
                    &pool_addresses,
                    &raw_tx,
                )
                .await;

                match result {
                    Ok(()) => {
                        warn!(endpoint = %endpoint, "yellowstone stream ended cleanly — reconnecting");
                        attempt = 0;
                        backoff_ms = INITIAL_BACKOFF_MS;
                    }
                    Err(e) => {
                        warn!(endpoint = %endpoint, attempt, error = %e, "yellowstone stream error");
                        attempt += 1;
                        if attempt >= MAX_RECONNECT_ATTEMPTS {
                            tracing::error!(
                                endpoint = %endpoint,
                                "yellowstone: exhausted {} reconnect attempts — giving up",
                                MAX_RECONNECT_ATTEMPTS
                            );
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms = (backoff_ms * 2).min(30_000);
                    }
                }
            }
        });
    });
}

#[cfg(feature = "yellowstone")]
async fn connect_and_stream(
    endpoint: &str,
    token: Option<&str>,
    pool_addresses: &[String],
    raw_tx: &Sender<RawUpdate>,
) -> anyhow::Result<()> {
    use anyhow::Context;
    use futures_util::StreamExt;
    use std::collections::HashMap;
    use yellowstone_grpc_client::GeyserGrpcClient;
    use yellowstone_grpc_proto::geyser::{
        CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
    };

    let mut client = GeyserGrpcClient::build_from_shared(endpoint.to_owned())?
        .x_token(token)?
        .connect()
        .await
        .context("grpc connect")?;

    // Build account filter: subscribe to the supplied pool addresses.
    let mut accounts_filter = HashMap::new();
    accounts_filter.insert(
        "pools".to_owned(),
        SubscribeRequestFilterAccounts {
            account: pool_addresses.to_vec(),
            owner: vec![],
            filters: vec![],
            nonempty_txn_signature: None,
        },
    );

    let request = SubscribeRequest {
        accounts: accounts_filter,
        slots: HashMap::new(),
        transactions: HashMap::new(),
        transactions_status: HashMap::new(),
        blocks: HashMap::new(),
        blocks_meta: HashMap::new(),
        entry: HashMap::new(),
        commitment: Some(CommitmentLevel::Confirmed as i32),
        accounts_data_slice: vec![],
        ping: None,
    };

    let (_, mut stream) = client.subscribe_with_request(Some(request)).await?;

    while let Some(msg) = stream.next().await {
        let msg = msg?;
        if let Some(yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof::Account(
            account_update,
        )) = msg.update_oneof
        {
            let account = match account_update.account {
                Some(a) => a,
                None => continue,
            };

            let address = bs58::encode(&account.pubkey).into_string();
            raw_tx
                .send(RawUpdate {
                    slot: account_update.slot,
                    signature: address.clone(),
                    logs: vec![],
                    accounts: vec![address],
                    timestamp: now_ms(),
                })
                .ok(); // drop if receiver gone
        }
    }

    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_returns_none_when_unset() {
        // Ensure the key is absent for this test.
        std::env::remove_var("GEYSER_GRPC_ENDPOINT");
        std::env::remove_var("YELLOWSTONE_ENDPOINT");
        assert!(GeyserAdapter::from_env().is_none());
    }

    #[test]
    fn from_env_parses_endpoint() {
        std::env::set_var("GEYSER_GRPC_ENDPOINT", "https://example.com:10000");
        let adapter = GeyserAdapter::from_env().expect("adapter created");
        assert_eq!(adapter.endpoint, "https://example.com:10000");
        std::env::remove_var("GEYSER_GRPC_ENDPOINT");
    }

    #[tokio::test]
    async fn pool_price_channel_is_usable() {
        let (tx, mut rx) = pool_price_channel();
        let event = PoolPrice {
            pool_address: "pool123".into(),
            dex: "orca_whirlpool".into(),
            price: 1.5,
            liquidity: 1_000_000,
            timestamp_ms: 1_700_000_000_000,
        };
        tx.send(event.clone()).unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.pool_address, event.pool_address);
        assert!((received.price - 1.5).abs() < 1e-12);
    }

    #[test]
    fn with_pools_attaches_addresses() {
        std::env::set_var("GEYSER_GRPC_ENDPOINT", "https://example.com:10000");
        let adapter = GeyserAdapter::from_env()
            .unwrap()
            .with_pools(vec!["pool1".into(), "pool2".into()]);
        assert_eq!(adapter.pool_addresses.len(), 2);
        std::env::remove_var("GEYSER_GRPC_ENDPOINT");
    }
}
