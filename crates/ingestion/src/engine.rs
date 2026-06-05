//! Async ingestion engine that publishes market events to the event bus.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use common::{MarketEvent, PoolUpdate, Result};
use events::EventBus;
use tokio::task::JoinHandle;
use tracing::{debug, trace};

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
        .map_or(0, |d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use common::MarketEvent;
    use events::EventBus;

    use super::{IngestionConfig, IngestionEngine};

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
}
