//! Real-time Solana ingestion boundary.
//!
//! This crate will own subscription streams and hand raw updates to decoders.

#![forbid(unsafe_code)]

pub mod ingestion {
    //! Stream setup, lifecycle, and update handoff responsibilities.

    use common::{Error, Pubkey, Result};
    use rpc_client::RpcClient;
    use tokio::sync::mpsc;
    use tracing::{debug, trace, warn};

    /// Default number of account updates buffered between RPC ingestion and consumers.
    pub const DEFAULT_CHANNEL_CAPACITY: usize = 4_096;

    /// Slot associated with an ingested Solana update.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Slot(u64);

    impl Slot {
        /// Creates a slot wrapper.
        #[must_use]
        pub const fn new(value: u64) -> Self {
            Self(value)
        }

        /// Returns the raw slot value.
        #[must_use]
        pub const fn get(self) -> u64 {
            self.0
        }
    }

    /// Commitment requested for a stream subscription.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub enum CommitmentLevel {
        /// Processed commitment.
        #[default]
        Processed,
        /// Confirmed commitment.
        Confirmed,
        /// Finalized commitment.
        Finalized,
    }

    /// Stream subscription target.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum SubscriptionKind {
        /// Subscribe to a single account.
        Account(Pubkey),
        /// Subscribe to all accounts owned by a program.
        Program(Pubkey),
        /// Subscribe to slot updates.
        Slots,
    }

    /// Subscription request metadata.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Subscription {
        kind: SubscriptionKind,
        commitment: CommitmentLevel,
    }

    impl Subscription {
        /// Creates a subscription request.
        #[must_use]
        pub const fn new(kind: SubscriptionKind, commitment: CommitmentLevel) -> Self {
            Self { kind, commitment }
        }

        /// Returns the subscription target.
        #[must_use]
        pub const fn kind(self) -> SubscriptionKind {
            self.kind
        }

        /// Returns the requested commitment.
        #[must_use]
        pub const fn commitment(self) -> CommitmentLevel {
            self.commitment
        }
    }

    /// Stream ingestion configuration.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct StreamConfig {
        channel_capacity: usize,
        commitment: CommitmentLevel,
    }

    impl StreamConfig {
        /// Creates stream configuration with a bounded update channel.
        pub fn new(channel_capacity: usize, commitment: CommitmentLevel) -> Result<Self> {
            if channel_capacity == 0 {
                return Err(Error::InvalidState(
                    "stream channel capacity must be greater than zero".to_owned(),
                ));
            }

            Ok(Self {
                channel_capacity,
                commitment,
            })
        }

        /// Returns the bounded update channel capacity.
        #[must_use]
        pub const fn channel_capacity(self) -> usize {
            self.channel_capacity
        }

        /// Returns the default commitment for subscriptions.
        #[must_use]
        pub const fn commitment(self) -> CommitmentLevel {
            self.commitment
        }
    }

    impl Default for StreamConfig {
        fn default() -> Self {
            Self {
                channel_capacity: DEFAULT_CHANNEL_CAPACITY,
                commitment: CommitmentLevel::Processed,
            }
        }
    }

    /// Raw account data update accepted by the ingestion layer.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AccountUpdate {
        pubkey: Pubkey,
        slot: Slot,
        write_version: u64,
        data: Vec<u8>,
    }

    impl AccountUpdate {
        /// Creates an account update from owned bytes without copying.
        #[must_use]
        pub fn from_vec(pubkey: Pubkey, slot: Slot, write_version: u64, data: Vec<u8>) -> Self {
            Self {
                pubkey,
                slot,
                write_version,
                data,
            }
        }

        /// Creates an account update by copying bytes from a slice.
        #[must_use]
        pub fn from_slice(pubkey: Pubkey, slot: Slot, write_version: u64, data: &[u8]) -> Self {
            Self::from_vec(pubkey, slot, write_version, data.to_vec())
        }

        /// Returns the updated account public key.
        #[must_use]
        pub const fn pubkey(&self) -> Pubkey {
            self.pubkey
        }

        /// Returns the update slot.
        #[must_use]
        pub const fn slot(&self) -> Slot {
            self.slot
        }

        /// Returns the write version emitted by upstream ingestion.
        #[must_use]
        pub const fn write_version(&self) -> u64 {
            self.write_version
        }

        /// Returns account data bytes.
        #[must_use]
        pub fn data(&self) -> &[u8] {
            &self.data
        }

        /// Consumes the update and returns owned account data.
        #[must_use]
        pub fn into_data(self) -> Vec<u8> {
            self.data
        }
    }

    /// Receives account updates from the ingestion queue.
    #[derive(Debug)]
    pub struct StreamReceiver {
        receiver: mpsc::Receiver<AccountUpdate>,
    }

    impl StreamReceiver {
        /// Receives the next account update.
        pub async fn recv(&mut self) -> Result<AccountUpdate> {
            match self.receiver.recv().await {
                Some(update) => {
                    trace!(
                        slot = update.slot().get(),
                        write_version = update.write_version(),
                        "received account update"
                    );
                    Ok(update)
                }
                None => {
                    warn!("stream receiver closed");
                    Err(Error::InternalError("stream queue is closed".to_owned()))
                }
            }
        }
    }

    /// Stream ingestion handle.
    #[derive(Clone, Debug)]
    pub struct StreamIngestor {
        rpc_client: RpcClient,
        config: StreamConfig,
        sender: mpsc::Sender<AccountUpdate>,
    }

    impl StreamIngestor {
        /// Creates a stream ingestor and receiver with default configuration.
        #[must_use]
        pub fn new(rpc_client: RpcClient) -> (Self, StreamReceiver) {
            Self::with_config(rpc_client, StreamConfig::default()).expect("valid config")
        }

        /// Creates a stream ingestor and paired receiver.
        pub fn with_config(
            rpc_client: RpcClient,
            config: StreamConfig,
        ) -> Result<(Self, StreamReceiver)> {
            rpc_client.ready()?;

            let (sender, receiver) = mpsc::channel(config.channel_capacity());
            debug!(
                channel_capacity = config.channel_capacity(),
                commitment = ?config.commitment(),
                "created stream ingestor"
            );

            Ok((
                Self {
                    rpc_client,
                    config,
                    sender,
                },
                StreamReceiver { receiver },
            ))
        }

        /// Registers a subscription request with the ingestion layer.
        pub fn register_subscription(&self, subscription: Subscription) -> Result<()> {
            self.ready()?;
            debug!(
                kind = ?subscription.kind(),
                commitment = ?subscription.commitment(),
                "registered stream subscription"
            );
            Ok(())
        }

        /// Enqueues an account update for downstream consumers.
        pub fn enqueue_account_update(&self, update: AccountUpdate) -> Result<()> {
            let slot = update.slot().get();
            let write_version = update.write_version();

            match self.sender.try_send(update) {
                Ok(()) => {
                    trace!(slot, write_version, "enqueued account update");
                    Ok(())
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    warn!(slot, write_version, "stream queue full");
                    Err(Error::InvalidState("stream queue is full".to_owned()))
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    warn!(slot, write_version, "stream queue closed");
                    Err(Error::InternalError("stream queue is closed".to_owned()))
                }
            }
        }

        /// Returns the RPC boundary used by ingestion.
        #[must_use]
        pub const fn rpc_client(&self) -> &RpcClient {
            &self.rpc_client
        }

        /// Returns the stream configuration.
        #[must_use]
        pub const fn config(&self) -> StreamConfig {
            self.config
        }

        /// Performs a readiness check for the stream boundary.
        pub fn ready(&self) -> Result<()> {
            self.rpc_client.ready()
        }
    }
}

pub use ingestion::{
    AccountUpdate, CommitmentLevel, Slot, StreamConfig, StreamIngestor, StreamReceiver,
    Subscription, SubscriptionKind, DEFAULT_CHANNEL_CAPACITY,
};

#[cfg(test)]
mod tests {
    use super::{
        AccountUpdate, CommitmentLevel, Slot, StreamConfig, StreamIngestor, Subscription,
        SubscriptionKind,
    };
    use common::{Error, Pubkey};
    use rpc_client::RpcClient;

    #[test]
    fn stream_config_rejects_zero_capacity() {
        let err = StreamConfig::new(0, CommitmentLevel::Processed).expect_err("invalid capacity");

        assert!(matches!(err, Error::InvalidState(_)));
    }

    #[test]
    fn stream_ingestor_is_ready() {
        let (ingestor, _receiver) = StreamIngestor::new(RpcClient::new());

        assert!(ingestor.ready().is_ok());
        assert!(ingestor.rpc_client().ready().is_ok());
    }

    #[test]
    fn stream_ingestor_registers_subscription() {
        let (ingestor, _receiver) = StreamIngestor::new(RpcClient::new());
        let subscription = Subscription::new(
            SubscriptionKind::Account(Pubkey::new([1; 32])),
            CommitmentLevel::Confirmed,
        );

        assert!(ingestor.register_subscription(subscription).is_ok());
    }

    #[tokio::test]
    async fn stream_ingestor_enqueues_and_receives_update() {
        let config = StreamConfig::new(8, CommitmentLevel::Processed).expect("valid config");
        let (ingestor, mut receiver) =
            StreamIngestor::with_config(RpcClient::new(), config).expect("ingestor");
        let update = AccountUpdate::from_slice(Pubkey::new([2; 32]), Slot::new(42), 7, &[1, 2, 3]);

        ingestor
            .enqueue_account_update(update.clone())
            .expect("enqueue update");

        assert_eq!(receiver.recv().await.expect("receive update"), update);
    }

    #[tokio::test]
    async fn stream_ingestor_reports_closed_receiver() {
        let config = StreamConfig::new(1, CommitmentLevel::Processed).expect("valid config");
        let (ingestor, mut receiver) =
            StreamIngestor::with_config(RpcClient::new(), config).expect("ingestor");

        drop(ingestor);

        let err = receiver.recv().await.expect_err("closed stream");
        assert!(matches!(err, Error::InternalError(_)));
    }

    #[test]
    fn stream_ingestor_reports_full_queue() {
        let config = StreamConfig::new(1, CommitmentLevel::Processed).expect("valid config");
        let (ingestor, _receiver) =
            StreamIngestor::with_config(RpcClient::new(), config).expect("ingestor");
        let update = AccountUpdate::from_vec(Pubkey::default(), Slot::new(1), 1, Vec::new());

        ingestor
            .enqueue_account_update(update.clone())
            .expect("first update");
        let err = ingestor
            .enqueue_account_update(update)
            .expect_err("queue full");

        assert!(matches!(err, Error::InvalidState(_)));
    }
}
