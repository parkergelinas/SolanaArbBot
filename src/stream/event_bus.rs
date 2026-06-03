use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError, TrySendError};
use dashmap::DashMap;
use thiserror::Error;

use super::event::MarketEvent;

const DEFAULT_SUBSCRIBER_QUEUE_CAPACITY: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackpressurePolicy {
    DropNewestForSlowConsumers,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventBusConfig {
    pub subscriber_queue_capacity: usize,
    pub backpressure_policy: BackpressurePolicy,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            subscriber_queue_capacity: DEFAULT_SUBSCRIBER_QUEUE_CAPACITY,
            backpressure_policy: BackpressurePolicy::DropNewestForSlowConsumers,
        }
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum EventBusError {
    #[error("subscriber queue capacity must be greater than zero")]
    InvalidCapacity,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PublishOutcome {
    pub subscribers: usize,
    pub delivered: usize,
    pub dropped: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EventBusStats {
    pub subscribers: usize,
    pub published_events: u64,
    pub delivered_events: u64,
    pub dropped_events: u64,
}

#[derive(Clone, Debug)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

#[derive(Debug)]
struct EventBusInner {
    config: EventBusConfig,
    next_subscriber_id: AtomicUsize,
    subscribers: DashMap<usize, Sender<MarketEvent>>,
    published_events: AtomicU64,
    delivered_events: AtomicU64,
    dropped_events: AtomicU64,
}

#[derive(Debug)]
pub struct EventBusReceiver {
    subscriber_id: usize,
    receiver: Receiver<MarketEvent>,
    inner: Arc<EventBusInner>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::with_config(EventBusConfig::default())
            .expect("default event bus config must be valid")
    }

    pub fn with_config(config: EventBusConfig) -> Result<Self, EventBusError> {
        if config.subscriber_queue_capacity == 0 {
            return Err(EventBusError::InvalidCapacity);
        }

        Ok(Self {
            inner: Arc::new(EventBusInner {
                config,
                next_subscriber_id: AtomicUsize::new(1),
                subscribers: DashMap::new(),
                published_events: AtomicU64::new(0),
                delivered_events: AtomicU64::new(0),
                dropped_events: AtomicU64::new(0),
            }),
        })
    }

    pub fn subscribe(&self) -> EventBusReceiver {
        let (sender, receiver) = bounded(self.inner.config.subscriber_queue_capacity);
        let subscriber_id = self
            .inner
            .next_subscriber_id
            .fetch_add(1, Ordering::Relaxed);

        self.inner.subscribers.insert(subscriber_id, sender);

        EventBusReceiver {
            subscriber_id,
            receiver,
            inner: Arc::clone(&self.inner),
        }
    }

    pub fn try_publish(&self, event: MarketEvent) -> PublishOutcome {
        self.inner.published_events.fetch_add(1, Ordering::Relaxed);

        match self.inner.config.backpressure_policy {
            BackpressurePolicy::DropNewestForSlowConsumers => {
                self.publish_drop_newest_for_slow_consumers(event)
            }
        }
    }

    pub fn stats(&self) -> EventBusStats {
        EventBusStats {
            subscribers: self.inner.subscribers.len(),
            published_events: self.inner.published_events.load(Ordering::Relaxed),
            delivered_events: self.inner.delivered_events.load(Ordering::Relaxed),
            dropped_events: self.inner.dropped_events.load(Ordering::Relaxed),
        }
    }

    fn publish_drop_newest_for_slow_consumers(&self, event: MarketEvent) -> PublishOutcome {
        let mut outcome = PublishOutcome {
            subscribers: self.inner.subscribers.len(),
            delivered: 0,
            dropped: 0,
        };
        let mut disconnected = Vec::new();

        for subscriber in self.inner.subscribers.iter() {
            match subscriber.value().try_send(event.clone()) {
                Ok(()) => outcome.delivered += 1,
                Err(TrySendError::Full(_)) => outcome.dropped += 1,
                Err(TrySendError::Disconnected(_)) => disconnected.push(*subscriber.key()),
            }
        }

        for subscriber_id in disconnected {
            self.inner.subscribers.remove(&subscriber_id);
        }

        if outcome.delivered > 0 {
            self.inner
                .delivered_events
                .fetch_add(outcome.delivered as u64, Ordering::Relaxed);
        }

        if outcome.dropped > 0 {
            self.inner
                .dropped_events
                .fetch_add(outcome.dropped as u64, Ordering::Relaxed);
        }

        outcome
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBusReceiver {
    pub fn subscriber_id(&self) -> usize {
        self.subscriber_id
    }

    pub fn recv(&self) -> Result<MarketEvent, crossbeam_channel::RecvError> {
        self.receiver.recv()
    }

    pub fn try_recv(&self) -> Result<MarketEvent, TryRecvError> {
        self.receiver.try_recv()
    }

    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }
}

impl Drop for EventBusReceiver {
    fn drop(&mut self) {
        self.inner.subscribers.remove(&self.subscriber_id);
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use super::*;
    use crate::stream::event::{EventMeta, EventSource, PoolUpdate};

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

    #[test]
    fn publish_fans_out_to_multiple_subscribers() {
        let bus = EventBus::new();
        let first = bus.subscribe();
        let second = bus.subscribe();

        let outcome = bus.try_publish(test_event(1));

        assert_eq!(
            outcome,
            PublishOutcome {
                subscribers: 2,
                delivered: 2,
                dropped: 0
            }
        );
        assert_eq!(first.recv().unwrap(), test_event(1));
        assert_eq!(second.recv().unwrap(), test_event(1));
    }

    #[test]
    fn publish_drops_newest_for_slow_subscriber() {
        let bus = EventBus::with_config(EventBusConfig {
            subscriber_queue_capacity: 1,
            backpressure_policy: BackpressurePolicy::DropNewestForSlowConsumers,
        })
        .unwrap();
        let subscriber = bus.subscribe();

        assert_eq!(bus.try_publish(test_event(1)).dropped, 0);
        assert_eq!(bus.try_publish(test_event(2)).dropped, 1);
        assert_eq!(subscriber.recv().unwrap(), test_event(1));
        assert_eq!(bus.stats().dropped_events, 1);
    }

    #[test]
    fn supports_multiple_concurrent_producers() {
        let bus = EventBus::new();
        let subscriber = bus.subscribe();
        let producer_count = 4;
        let events_per_producer = 128;

        thread::scope(|scope| {
            for producer in 0..producer_count {
                let bus = bus.clone();
                scope.spawn(move || {
                    for offset in 0..events_per_producer {
                        let slot = producer * events_per_producer + offset;
                        bus.try_publish(test_event(slot as u64));
                    }
                });
            }
        });

        let expected = producer_count * events_per_producer;
        let mut received = 0;
        while received < expected {
            subscriber.recv().unwrap();
            received += 1;
        }

        assert_eq!(bus.stats().delivered_events, expected as u64);
    }

    #[test]
    fn dropping_receiver_unregisters_subscriber() {
        let bus = EventBus::new();
        let subscriber = bus.subscribe();
        assert_eq!(bus.stats().subscribers, 1);

        drop(subscriber);

        assert_eq!(bus.stats().subscribers, 0);
    }

    #[test]
    fn rejects_zero_capacity() {
        let error = EventBus::with_config(EventBusConfig {
            subscriber_queue_capacity: 0,
            backpressure_policy: BackpressurePolicy::DropNewestForSlowConsumers,
        })
        .unwrap_err();

        assert_eq!(error, EventBusError::InvalidCapacity);
    }
}
