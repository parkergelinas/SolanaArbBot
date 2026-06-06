//! Market events and real-time event bus.
//!
//! This crate is the canonical home for:
//! - **Market event domain types** ([`MarketEvent`], [`PoolUpdate`], [`SwapEvent`],
//!   [`TickUpdate`]) — re-exported from [`common`] for convenience.
//! - **[`EventBus`] infrastructure** — multi-producer, multi-consumer fan-out bus
//!   with configurable backpressure.
//!
//! The legacy [`stream`] crate re-exports everything from this crate for
//! backward compatibility.

#![forbid(unsafe_code)]

pub mod bus;

// Re-export market event types so consumers can import from a single place.
pub use common::{MarketEvent, PoolUpdate, PriceUpdate, SwapEvent, TickUpdate};

pub use bus::{
    BackpressureStrategy, EventBus, EventBusConfig, EventSubscriber, PublishReport,
    SubscriberId, DEFAULT_CHANNEL_CAPACITY,
};
