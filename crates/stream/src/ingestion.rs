use std::time::Duration;

use rpc_client::{MarketRpcClient, RpcClientError};
use thiserror::Error;
use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::event_bus::{EventBus, PublishOutcome};

const DEFAULT_RECONNECT_INITIAL_DELAY: Duration = Duration::from_millis(100);
const DEFAULT_RECONNECT_MAX_DELAY: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IngestionConfig {
    pub reconnect_initial_delay: Duration,
    pub reconnect_max_delay: Duration,
    pub max_reconnect_attempts: Option<usize>,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            reconnect_initial_delay: DEFAULT_RECONNECT_INITIAL_DELAY,
            reconnect_max_delay: DEFAULT_RECONNECT_MAX_DELAY,
            max_reconnect_attempts: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IngestionStats {
    pub connection_attempts: u64,
    pub successful_connections: u64,
    pub reconnects: u64,
    pub stream_failures: u64,
    pub events_received: u64,
    pub events_delivered: u64,
    pub events_dropped: u64,
    pub backpressure_batches: u64,
}

#[derive(Debug, Error)]
pub enum IngestionError {
    #[error("rpc client error")]
    Rpc(#[from] RpcClientError),

    #[error("maximum reconnect attempts exceeded: {attempts}")]
    ReconnectAttemptsExceeded {
        attempts: usize,
        #[source]
        last_error: RpcClientError,
    },
}

#[derive(Debug)]
pub struct MarketIngestion<C> {
    client: C,
    event_bus: EventBus,
    config: IngestionConfig,
}

impl<C> MarketIngestion<C>
where
    C: MarketRpcClient,
{
    pub fn new(client: C, event_bus: EventBus, config: IngestionConfig) -> Self {
        Self {
            client,
            event_bus,
            config,
        }
    }

    pub fn with_default_config(client: C, event_bus: EventBus) -> Self {
        Self::new(client, event_bus, IngestionConfig::default())
    }

    pub async fn run_until_shutdown(
        mut self,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<IngestionStats, IngestionError> {
        let mut stats = IngestionStats::default();
        let mut reconnect_delay = self.config.reconnect_initial_delay;
        let mut reconnect_attempts = 0_usize;

        loop {
            if Self::shutdown_requested(&shutdown) {
                return Ok(stats);
            }

            stats.connection_attempts += 1;
            match self.client.connect().await {
                Ok(()) => {
                    stats.successful_connections += 1;
                    info!(
                        connection_attempts = stats.connection_attempts,
                        "market ingestion stream connected"
                    );
                }
                Err(error) if error.is_recoverable() => {
                    stats.reconnects += 1;
                    reconnect_attempts += 1;
                    warn!(?error, "market ingestion stream connect failed");
                    Self::check_reconnect_limit(
                        self.config.max_reconnect_attempts,
                        reconnect_attempts,
                        error,
                    )?;

                    if Self::wait_for_reconnect_delay(reconnect_delay, &mut shutdown).await {
                        return Ok(stats);
                    }
                    reconnect_delay = Self::next_reconnect_delay(
                        reconnect_delay,
                        self.config.reconnect_max_delay,
                    );
                    continue;
                }
                Err(error) => return Err(IngestionError::Rpc(error)),
            }

            loop {
                tokio::select! {
                    shutdown_changed = shutdown.changed() => {
                        if shutdown_changed.is_err() || Self::shutdown_requested(&shutdown) {
                            return Ok(stats);
                        }
                    }
                    next_event = self.client.next_event() => {
                        match next_event {
                            Ok(event) => {
                                stats.events_received += 1;
                                let outcome = self.event_bus.try_publish(event);
                                Self::record_publish_outcome(&mut stats, outcome);
                                reconnect_attempts = 0;
                                reconnect_delay = self.config.reconnect_initial_delay;
                            }
                            Err(error) if error.is_recoverable() => {
                                stats.stream_failures += 1;
                                stats.reconnects += 1;
                                reconnect_attempts += 1;
                                warn!(?error, "market ingestion stream failed; reconnecting");
                                Self::check_reconnect_limit(
                                    self.config.max_reconnect_attempts,
                                    reconnect_attempts,
                                    error,
                                )?;

                                if Self::wait_for_reconnect_delay(reconnect_delay, &mut shutdown).await {
                                    return Ok(stats);
                                }
                                reconnect_delay = Self::next_reconnect_delay(
                                    reconnect_delay,
                                    self.config.reconnect_max_delay,
                                );
                                break;
                            }
                            Err(error) => return Err(IngestionError::Rpc(error)),
                        }
                    }
                }
            }
        }
    }

    fn record_publish_outcome(stats: &mut IngestionStats, outcome: PublishOutcome) {
        stats.events_delivered += outcome.delivered as u64;
        stats.events_dropped += outcome.dropped as u64;

        if outcome.dropped > 0 {
            stats.backpressure_batches += 1;
            debug!(
                subscribers = outcome.subscribers,
                delivered = outcome.delivered,
                dropped = outcome.dropped,
                "market ingestion dropped events for slow subscribers"
            );
        }
    }

    fn check_reconnect_limit(
        max_reconnect_attempts: Option<usize>,
        reconnect_attempts: usize,
        last_error: RpcClientError,
    ) -> Result<(), IngestionError> {
        if max_reconnect_attempts.is_some_and(|max| reconnect_attempts > max) {
            return Err(IngestionError::ReconnectAttemptsExceeded {
                attempts: reconnect_attempts,
                last_error,
            });
        }

        Ok(())
    }

    async fn wait_for_reconnect_delay(
        reconnect_delay: Duration,
        shutdown: &mut watch::Receiver<bool>,
    ) -> bool {
        if reconnect_delay.is_zero() {
            return Self::shutdown_requested(shutdown);
        }

        tokio::select! {
            _ = tokio::time::sleep(reconnect_delay) => Self::shutdown_requested(shutdown),
            shutdown_changed = shutdown.changed() => {
                shutdown_changed.is_err() || Self::shutdown_requested(shutdown)
            }
        }
    }

    fn next_reconnect_delay(current: Duration, max: Duration) -> Duration {
        current.saturating_mul(2).min(max)
    }

    fn shutdown_requested(shutdown: &watch::Receiver<bool>) -> bool {
        *shutdown.borrow()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        future::pending,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use common::{EventMeta, EventSource, MarketEvent, PoolUpdate};
    use tokio::{sync::watch, time::timeout};

    use super::*;
    use crate::event_bus::{BackpressurePolicy, EventBusConfig, EventBusReceiver};

    enum ScriptStep {
        Event(MarketEvent),
        Error(RpcClientError),
        Pending,
    }

    struct ScriptedRpcClient {
        steps: VecDeque<ScriptStep>,
        connect_calls: Arc<AtomicUsize>,
    }

    impl ScriptedRpcClient {
        fn new(steps: impl IntoIterator<Item = ScriptStep>) -> (Self, Arc<AtomicUsize>) {
            let connect_calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    steps: steps.into_iter().collect(),
                    connect_calls: Arc::clone(&connect_calls),
                },
                connect_calls,
            )
        }
    }

    impl MarketRpcClient for ScriptedRpcClient {
        async fn connect(&mut self) -> Result<(), RpcClientError> {
            self.connect_calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn next_event(&mut self) -> Result<MarketEvent, RpcClientError> {
            match self.steps.pop_front() {
                Some(ScriptStep::Event(event)) => Ok(event),
                Some(ScriptStep::Error(error)) => Err(error),
                Some(ScriptStep::Pending) | None => pending().await,
            }
        }
    }

    fn test_event(slot: u64) -> MarketEvent {
        MarketEvent::PoolUpdate(PoolUpdate {
            meta: EventMeta {
                slot,
                source: EventSource::AccountStream,
                received_at_unix_nanos: slot,
            },
            pool: [1; 32],
            program: [2; 32],
            account_data: Arc::from([slot as u8]),
        })
    }

    fn test_config() -> IngestionConfig {
        IngestionConfig {
            reconnect_initial_delay: Duration::from_millis(1),
            reconnect_max_delay: Duration::from_millis(1),
            max_reconnect_attempts: Some(3),
        }
    }

    async fn recv_event(subscriber: &EventBusReceiver) -> MarketEvent {
        timeout(Duration::from_secs(1), async {
            loop {
                match subscriber.try_recv() {
                    Ok(event) => break event,
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        panic!("event bus subscriber disconnected")
                    }
                }
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn publishes_rpc_events_to_event_bus() {
        let (client, _connect_calls) =
            ScriptedRpcClient::new([ScriptStep::Event(test_event(1)), ScriptStep::Pending]);
        let event_bus = EventBus::new();
        let subscriber = event_bus.subscribe();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let ingestion = MarketIngestion::new(client, event_bus, test_config());

        let handle = tokio::spawn(ingestion.run_until_shutdown(shutdown_rx));

        assert_eq!(recv_event(&subscriber).await, test_event(1));
        shutdown_tx.send(true).unwrap();

        let stats = timeout(Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(stats.events_received, 1);
        assert_eq!(stats.events_delivered, 1);
        assert_eq!(stats.events_dropped, 0);
    }

    #[tokio::test]
    async fn reconnects_after_recoverable_stream_failure() {
        let (client, connect_calls) = ScriptedRpcClient::new([
            ScriptStep::Event(test_event(1)),
            ScriptStep::Error(RpcClientError::Disconnected),
            ScriptStep::Event(test_event(2)),
            ScriptStep::Pending,
        ]);
        let event_bus = EventBus::new();
        let subscriber = event_bus.subscribe();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let ingestion = MarketIngestion::new(client, event_bus, test_config());

        let handle = tokio::spawn(ingestion.run_until_shutdown(shutdown_rx));

        assert_eq!(recv_event(&subscriber).await, test_event(1));
        assert_eq!(recv_event(&subscriber).await, test_event(2));
        shutdown_tx.send(true).unwrap();

        let stats = timeout(Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(connect_calls.load(Ordering::Relaxed), 2);
        assert_eq!(stats.reconnects, 1);
        assert_eq!(stats.stream_failures, 1);
        assert_eq!(stats.events_delivered, 2);
    }

    #[tokio::test]
    async fn records_backpressure_drops_from_event_bus() {
        let (client, _connect_calls) = ScriptedRpcClient::new([
            ScriptStep::Event(test_event(1)),
            ScriptStep::Event(test_event(2)),
            ScriptStep::Pending,
        ]);
        let event_bus = EventBus::with_config(EventBusConfig {
            subscriber_queue_capacity: 1,
            backpressure_policy: BackpressurePolicy::DropNewestForSlowConsumers,
        })
        .unwrap();
        let _subscriber = event_bus.subscribe();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let ingestion = MarketIngestion::new(client, event_bus.clone(), test_config());

        let handle = tokio::spawn(ingestion.run_until_shutdown(shutdown_rx));
        timeout(Duration::from_secs(1), async {
            loop {
                if event_bus.stats().dropped_events == 1 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
        shutdown_tx.send(true).unwrap();

        let stats = timeout(Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(stats.events_received, 2);
        assert_eq!(stats.events_delivered, 1);
        assert_eq!(stats.events_dropped, 1);
        assert_eq!(stats.backpressure_batches, 1);
    }

    #[tokio::test]
    async fn stops_after_reconnect_limit() {
        let (client, _connect_calls) = ScriptedRpcClient::new([
            ScriptStep::Error(RpcClientError::Disconnected),
            ScriptStep::Error(RpcClientError::Disconnected),
        ]);
        let event_bus = EventBus::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);
        let ingestion = MarketIngestion::new(
            client,
            event_bus,
            IngestionConfig {
                reconnect_initial_delay: Duration::ZERO,
                reconnect_max_delay: Duration::ZERO,
                max_reconnect_attempts: Some(1),
            },
        );

        let error = ingestion.run_until_shutdown(shutdown_rx).await.unwrap_err();

        assert!(matches!(
            error,
            IngestionError::ReconnectAttemptsExceeded { attempts: 2, .. }
        ));
    }
}
