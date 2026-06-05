//! Backward-compatible re-export of the [`events`] and [`ingestion`] crates.
//!
//! All existing consumers that import from `stream` continue to work without
//! modification.  New code should import directly from `events` or `ingestion`.

#![forbid(unsafe_code)]

// ── Event bus ─────────────────────────────────────────────────────────────
pub use events::{
    BackpressureStrategy, EventBus, EventBusConfig, EventSubscriber, MarketEvent, PoolUpdate,
    PublishReport, SubscriberId, SwapEvent, TickUpdate, DEFAULT_CHANNEL_CAPACITY,
};

// ── Ingestion engine ──────────────────────────────────────────────────────
pub use ingestion::{
    IngestionConfig, IngestionEngine, DEFAULT_PLACEHOLDER_EVENT_LIMIT,
};

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
