//! Real-time market data streaming backbone.
//!
//! This crate owns the in-process event bus used to fan out decoded market
//! events to pricing, graph, routing, and simulation components.

#![forbid(unsafe_code)]

pub mod bus {
    //! Multi-producer, multi-consumer fan-out event bus.

    use std::{
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc,
        },
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use common::{Error, MarketEvent, Result};
    use crossbeam_channel::{
        bounded, Receiver, RecvTimeoutError, Sender, TryRecvError, TrySendError,
    };
    use dashmap::DashMap;
    use tracing::{debug, trace, warn};

    /// Default bounded channel size for each subscriber.
    pub const DEFAULT_CHANNEL_CAPACITY: usize = 4_096;

    /// Subscriber identifier assigned by the event bus.
    pub type SubscriberId = u64;

    /// Backpressure behavior when a subscriber channel is full.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum BackpressureStrategy {
        /// Drop the newest event for the full subscriber and continue fan-out.
        DropNewest,
        /// Block the publishing thread until each subscriber accepts the event.
        Block,
    }

    /// Event bus configuration.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct EventBusConfig {
        channel_capacity: usize,
        backpressure: BackpressureStrategy,
    }

    impl EventBusConfig {
        /// Creates event bus configuration.
        pub fn new(channel_capacity: usize, backpressure: BackpressureStrategy) -> Result<Self> {
            if channel_capacity == 0 {
                return Err(Error::InvalidState(
                    "event bus channel capacity must be greater than zero".to_owned(),
                ));
            }

            Ok(Self {
                channel_capacity,
                backpressure,
            })
        }

        /// Returns per-subscriber channel capacity.
        #[must_use]
        pub const fn channel_capacity(self) -> usize {
            self.channel_capacity
        }

        /// Returns configured backpressure behavior.
        #[must_use]
        pub const fn backpressure(self) -> BackpressureStrategy {
            self.backpressure
        }
    }

    impl Default for EventBusConfig {
        fn default() -> Self {
            Self {
                channel_capacity: DEFAULT_CHANNEL_CAPACITY,
                backpressure: BackpressureStrategy::DropNewest,
            }
        }
    }

    /// Fan-out publish result.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct PublishReport {
        delivered: usize,
        dropped: usize,
        disconnected: usize,
    }

    impl PublishReport {
        /// Number of subscriber channels that accepted the event.
        #[must_use]
        pub const fn delivered(self) -> usize {
            self.delivered
        }

        /// Number of subscriber channels that dropped the event due to backpressure.
        #[must_use]
        pub const fn dropped(self) -> usize {
            self.dropped
        }

        /// Number of closed subscriber channels removed during publish.
        #[must_use]
        pub const fn disconnected(self) -> usize {
            self.disconnected
        }
    }

    /// Multi-producer, multi-consumer market event bus.
    #[derive(Clone, Debug)]
    pub struct EventBus {
        config: EventBusConfig,
        next_subscriber_id: Arc<AtomicU64>,
        subscribers: Arc<DashMap<SubscriberId, Sender<MarketEvent>>>,
    }

    impl EventBus {
        /// Creates an event bus with default configuration.
        #[must_use]
        pub fn new() -> Self {
            Self::with_config(EventBusConfig::default()).expect("valid default config")
        }

        /// Creates an event bus with explicit configuration.
        pub fn with_config(config: EventBusConfig) -> Result<Self> {
            debug!(
                channel_capacity = config.channel_capacity(),
                backpressure = ?config.backpressure(),
                "created event bus"
            );
            Ok(Self {
                config,
                next_subscriber_id: Arc::new(AtomicU64::new(1)),
                subscribers: Arc::new(DashMap::new()),
            })
        }

        /// Registers a new subscriber and returns its receiver handle.
        pub fn subscribe(&self) -> EventSubscriber {
            let id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
            let (sender, receiver) = bounded(self.config.channel_capacity());
            self.subscribers.insert(id, sender);
            debug!(subscriber_id = id, "registered event subscriber");

            EventSubscriber {
                id,
                receiver,
                subscribers: Arc::clone(&self.subscribers),
            }
        }

        /// Publishes an event to every active subscriber.
        ///
        /// With [`BackpressureStrategy::Block`], this method may block the caller.
        /// Async ingestion code should call [`Self::publish_async`] instead.
        pub fn publish(&self, event: MarketEvent) -> Result<PublishReport> {
            let stage_started_at = Instant::now();
            let event_timestamp_micros = unix_timestamp_micros();
            let mut report = PublishReport::default();
            let mut disconnected = Vec::new();

            for subscriber in self.subscribers.iter() {
                match self.config.backpressure() {
                    BackpressureStrategy::DropNewest => {
                        match subscriber.value().try_send(event.clone()) {
                            Ok(()) => report.delivered += 1,
                            Err(TrySendError::Full(_)) => {
                                report.dropped += 1;
                                trace!(
                                    subscriber_id = *subscriber.key(),
                                    "dropped event for full subscriber channel"
                                );
                            }
                            Err(TrySendError::Disconnected(_)) => {
                                report.disconnected += 1;
                                disconnected.push(*subscriber.key());
                            }
                        }
                    }
                    BackpressureStrategy::Block => {
                        if subscriber.value().send(event.clone()).is_ok() {
                            report.delivered += 1;
                        } else {
                            report.disconnected += 1;
                            disconnected.push(*subscriber.key());
                        }
                    }
                }
            }

            for subscriber_id in disconnected {
                self.subscribers.remove(&subscriber_id);
                warn!(subscriber_id, "removed disconnected subscriber");
            }

            debug!(
                event_timestamp_micros,
                latency_micros = stage_started_at.elapsed().as_micros(),
                delivered = report.delivered(),
                dropped = report.dropped(),
                disconnected = report.disconnected(),
                subscribers = self.subscriber_count(),
                backpressure = ?self.config.backpressure(),
                "event bus publish complete"
            );

            Ok(report)
        }

        /// Publishes from async code without blocking the Tokio worker thread.
        pub async fn publish_async(&self, event: MarketEvent) -> Result<PublishReport> {
            let stage_started_at = Instant::now();
            let event_timestamp_micros = unix_timestamp_micros();
            match self.config.backpressure() {
                BackpressureStrategy::DropNewest => {
                    let report = self.publish(event)?;
                    trace!(
                        event_timestamp_micros,
                        latency_micros = stage_started_at.elapsed().as_micros(),
                        "event bus async publish complete"
                    );
                    Ok(report)
                }
                BackpressureStrategy::Block => {
                    let bus = self.clone();
                    let report = tokio::task::spawn_blocking(move || bus.publish(event))
                        .await
                        .map_err(|err| {
                            Error::InternalError(format!("event bus publish task failed: {err}"))
                        })??;
                    trace!(
                        event_timestamp_micros,
                        latency_micros = stage_started_at.elapsed().as_micros(),
                        "event bus async blocking publish complete"
                    );
                    Ok(report)
                }
            }
        }

        /// Returns the current number of active subscribers.
        #[must_use]
        pub fn subscriber_count(&self) -> usize {
            self.subscribers.len()
        }

        /// Returns event bus configuration.
        #[must_use]
        pub const fn config(&self) -> EventBusConfig {
            self.config
        }
    }

    impl Default for EventBus {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Receiver handle for one event-bus subscription.
    #[derive(Debug)]
    pub struct EventSubscriber {
        id: SubscriberId,
        receiver: Receiver<MarketEvent>,
        subscribers: Arc<DashMap<SubscriberId, Sender<MarketEvent>>>,
    }

    impl EventSubscriber {
        /// Returns this subscriber's bus id.
        #[must_use]
        pub const fn id(&self) -> SubscriberId {
            self.id
        }

        /// Blocks until the next event is available.
        pub fn recv(&self) -> Result<MarketEvent> {
            self.receiver
                .recv()
                .map_err(|err| Error::InternalError(format!("event subscriber closed: {err}")))
        }

        /// Receives the next event if one is immediately available.
        pub fn try_recv(&self) -> Result<Option<MarketEvent>> {
            match self.receiver.try_recv() {
                Ok(event) => Ok(Some(event)),
                Err(TryRecvError::Empty) => Ok(None),
                Err(TryRecvError::Disconnected) => Err(Error::InternalError(
                    "event subscriber disconnected".to_owned(),
                )),
            }
        }

        /// Blocks up to `timeout` while waiting for the next event.
        pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<MarketEvent>> {
            match self.receiver.recv_timeout(timeout) {
                Ok(event) => Ok(Some(event)),
                Err(RecvTimeoutError::Timeout) => Ok(None),
                Err(RecvTimeoutError::Disconnected) => Err(Error::InternalError(
                    "event subscriber disconnected".to_owned(),
                )),
            }
        }
    }

    impl Drop for EventSubscriber {
        fn drop(&mut self) {
            self.subscribers.remove(&self.id);
            debug!(subscriber_id = self.id, "dropped event subscriber");
        }
    }

    fn unix_timestamp_micros() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| {
                u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
            })
    }
}

pub mod ingestion {
    //! Placeholder Tokio ingestion task that publishes market events.

    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use common::{MarketEvent, PoolUpdate, Result};
    use tokio::task::JoinHandle;
    use tracing::{debug, trace};

    use crate::EventBus;

    /// Default number of placeholder events emitted by the ingestion task.
    pub const DEFAULT_PLACEHOLDER_EVENT_LIMIT: usize = 1_024;

    /// Ingestion task configuration.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct IngestionConfig {
        event_limit: usize,
        yield_every: usize,
    }

    impl IngestionConfig {
        /// Creates ingestion configuration for a placeholder source.
        #[must_use]
        pub const fn new(event_limit: usize, yield_every: usize) -> Self {
            Self {
                event_limit,
                yield_every,
            }
        }

        /// Returns the number of placeholder events to emit.
        #[must_use]
        pub const fn event_limit(self) -> usize {
            self.event_limit
        }

        /// Returns how often the ingestion task yields to the runtime.
        #[must_use]
        pub const fn yield_every(self) -> usize {
            self.yield_every
        }
    }

    impl Default for IngestionConfig {
        fn default() -> Self {
            Self {
                event_limit: DEFAULT_PLACEHOLDER_EVENT_LIMIT,
                yield_every: 256,
            }
        }
    }

    /// Async ingestion engine for a placeholder Solana market-data source.
    #[derive(Clone, Debug)]
    pub struct IngestionEngine {
        event_bus: EventBus,
        config: IngestionConfig,
    }

    impl IngestionEngine {
        /// Creates an ingestion engine.
        #[must_use]
        pub fn new(event_bus: EventBus, config: IngestionConfig) -> Self {
            Self { event_bus, config }
        }

        /// Starts the placeholder ingestion loop on the Tokio runtime.
        pub fn start(self) -> JoinHandle<Result<usize>> {
            tokio::spawn(async move { self.run().await })
        }

        /// Runs the placeholder ingestion loop to completion.
        pub async fn run(self) -> Result<usize> {
            debug!(
                event_limit = self.config.event_limit(),
                yield_every = self.config.yield_every(),
                "starting placeholder ingestion"
            );

            let mut published = 0;
            for index in 0..self.config.event_limit() {
                let stage_started_at = Instant::now();
                let event_timestamp_micros = unix_timestamp_micros();
                let event = placeholder_event();
                let report = self.event_bus.publish_async(event).await?;
                trace!(
                    index,
                    event_timestamp_micros,
                    delivered = report.delivered(),
                    dropped = report.dropped(),
                    latency_micros = stage_started_at.elapsed().as_micros(),
                    "stream ingestion published market event"
                );
                published += 1;

                if self.config.yield_every() != 0 && index % self.config.yield_every() == 0 {
                    tokio::task::yield_now().await;
                }
            }

            debug!(published, "finished placeholder ingestion");
            Ok(published)
        }

        /// Returns the event bus used by this ingestion engine.
        #[must_use]
        pub const fn event_bus(&self) -> &EventBus {
            &self.event_bus
        }

        /// Returns ingestion configuration.
        #[must_use]
        pub const fn config(&self) -> IngestionConfig {
            self.config
        }
    }

    fn placeholder_event() -> MarketEvent {
        MarketEvent::PoolUpdate(PoolUpdate {
            pool: None,
            token_a_mint: Some(common::Pubkey::new([1; 32])),
            token_b_mint: Some(common::Pubkey::new([2; 32])),
            liquidity: Some(100_000),
            sqrt_price: Some(1_000),
            fee_rate: Some(25),
        })
    }

    fn unix_timestamp_micros() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| {
                u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
            })
    }
}

pub use bus::{
    BackpressureStrategy, EventBus, EventBusConfig, EventSubscriber, PublishReport, SubscriberId,
    DEFAULT_CHANNEL_CAPACITY,
};
pub use ingestion::{IngestionConfig, IngestionEngine, DEFAULT_PLACEHOLDER_EVENT_LIMIT};

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use common::{MarketEvent, PoolUpdate, Pubkey, SwapEvent};
    use tracing::info;

    use super::{BackpressureStrategy, EventBus, EventBusConfig, IngestionConfig, IngestionEngine};

    #[test]
    fn event_bus_supports_multi_producer() {
        let bus = EventBus::with_config(
            EventBusConfig::new(256, BackpressureStrategy::DropNewest).expect("config"),
        )
        .expect("bus");
        let subscriber = bus.subscribe();

        let producers = (0..4)
            .map(|producer_id| {
                let bus = bus.clone();
                thread::spawn(move || {
                    for sequence in 0..25 {
                        bus.publish(swap_event(producer_id, sequence))
                            .expect("publish");
                    }
                })
            })
            .collect::<Vec<_>>();

        for producer in producers {
            producer.join().expect("producer joined");
        }

        let mut received = Vec::with_capacity(100);
        for _ in 0..100 {
            received.push(
                subscriber
                    .recv_timeout(Duration::from_secs(1))
                    .expect("receive")
                    .expect("event"),
            );
        }

        assert_eq!(received.len(), 100);
    }

    #[test]
    fn event_bus_fans_out_to_multi_consumer() {
        let bus = EventBus::new();
        let first = bus.subscribe();
        let second = bus.subscribe();
        let event = pool_event();

        let report = bus.publish(event.clone()).expect("publish");

        assert_eq!(report.delivered(), 2);
        assert_eq!(first.recv().expect("first event"), event);
        assert_eq!(second.recv().expect("second event"), event);
    }

    #[test]
    fn event_bus_preserves_message_integrity() {
        let bus = EventBus::new();
        let subscriber = bus.subscribe();
        let event = swap_event(7, 11);

        bus.publish(event.clone()).expect("publish");

        assert_eq!(subscriber.recv().expect("event"), event);
    }

    #[test]
    fn event_bus_drop_newest_reports_backpressure() {
        let bus = EventBus::with_config(
            EventBusConfig::new(1, BackpressureStrategy::DropNewest).expect("config"),
        )
        .expect("bus");
        let _subscriber = bus.subscribe();

        assert_eq!(bus.publish(pool_event()).expect("first").delivered(), 1);
        let report = bus.publish(pool_event()).expect("second");

        assert_eq!(report.delivered(), 0);
        assert_eq!(report.dropped(), 1);
    }

    #[tokio::test]
    async fn ingestion_engine_publishes_placeholder_events() {
        let bus = EventBus::new();
        let subscriber = bus.subscribe();
        let engine = IngestionEngine::new(bus, IngestionConfig::new(4, 1));

        let published = engine.run().await.expect("ingestion");

        assert_eq!(published, 4);
        for _ in 0..4 {
            assert!(matches!(
                subscriber.recv().expect("event"),
                MarketEvent::PoolUpdate(_)
            ));
        }
    }

    #[tokio::test]
    async fn mock_ingestion_propagates_high_frequency_events_without_loss() {
        const EVENT_COUNT: usize = 20_000;

        let bus = EventBus::with_config(
            EventBusConfig::new(EVENT_COUNT, BackpressureStrategy::DropNewest).expect("config"),
        )
        .expect("bus");
        let first = bus.subscribe();
        let second = bus.subscribe();
        let engine = IngestionEngine::new(bus, IngestionConfig::new(EVENT_COUNT, 0));

        let started_at = Instant::now();
        let published = engine.run().await.expect("ingestion");
        let elapsed = started_at.elapsed();
        let events_per_second = published as f64 / elapsed.as_secs_f64();

        info!(
            published,
            elapsed_ms = elapsed.as_secs_f64() * 1_000.0,
            events_per_second,
            "mock ingestion event propagation throughput"
        );
        eprintln!(
            "mock ingestion throughput: {published} events in {:.3} ms ({events_per_second:.2} events/sec)",
            elapsed.as_secs_f64() * 1_000.0
        );

        assert_eq!(published, EVENT_COUNT);
        assert_eq!(drain_events(&first, EVENT_COUNT), EVENT_COUNT);
        assert_eq!(drain_events(&second, EVENT_COUNT), EVENT_COUNT);
        assert!(first
            .recv_timeout(Duration::from_millis(10))
            .expect("first no extra")
            .is_none());
        assert!(second
            .recv_timeout(Duration::from_millis(10))
            .expect("second no extra")
            .is_none());
        assert!(events_per_second.is_finite());
        assert!(events_per_second > 0.0);
    }

    fn pool_event() -> MarketEvent {
        MarketEvent::PoolUpdate(PoolUpdate {
            pool: Some(Pubkey::new([1; 32])),
            token_a_mint: Some(Pubkey::new([2; 32])),
            token_b_mint: Some(Pubkey::new([3; 32])),
            liquidity: Some(42),
            sqrt_price: Some(64),
            fee_rate: Some(25),
        })
    }

    fn swap_event(producer_id: u8, sequence: u8) -> MarketEvent {
        MarketEvent::SwapEvent(SwapEvent {
            pool: Pubkey::new([producer_id; 32]),
            input_mint: Pubkey::new([sequence; 32]),
            output_mint: Pubkey::new([producer_id.wrapping_add(sequence); 32]),
            amount_in: u128::from(sequence),
            amount_out: u128::from(sequence) + 1,
        })
    }

    fn drain_events(subscriber: &super::EventSubscriber, expected: usize) -> usize {
        let mut received = 0;
        for _ in 0..expected {
            assert!(matches!(
                subscriber
                    .recv_timeout(Duration::from_secs(1))
                    .expect("receive event")
                    .expect("event"),
                MarketEvent::PoolUpdate(_)
            ));
            received += 1;
        }
        received
    }
}
