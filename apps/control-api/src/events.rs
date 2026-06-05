//! WebSocket event envelope broadcast to every connected client.

use serde::{Deserialize, Serialize};

use crate::dto::{HealthDto, PortfolioDto, RiskDto, SignalEventDto, SystemStateDto, TradeEventDto};

/// Discriminated union of all events the WebSocket stream can carry.
///
/// Serialises as `{ "type": "...", "data": { ... } }` via serde's
/// `tag` + `content` representation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum WsEvent {
    /// A new signal emitted by the signal engine.
    Signal(SignalEventDto),
    /// Trade lifecycle update (started → filled / rejected / …).
    Trade(TradeEventDto),
    /// Periodic system health heartbeat (emitted every 5 s).
    Health(HealthDto),
    /// System running-state change.
    Status(SystemStateDto),
    /// Portfolio metrics update.
    Portfolio(PortfolioDto),
    /// Risk exposure update.
    Risk(RiskDto),
    /// Notification that a config field was changed.
    ConfigChanged { section: String, summary: String },
}
