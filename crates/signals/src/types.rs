//! Core signal types produced and consumed by the signal engine.
//!
//! All types are `Clone + Debug` and carry no runtime state.  `SignalEvent` is
//! the canonical output of every pipeline run; `WhaleEvent` is the input
//! produced by callers who observe large-wallet activity.

use common::Pubkey;
use common::MarketEvent;

// ─────────────────────────────────────────────────────────────────────────────
// Direction
// ─────────────────────────────────────────────────────────────────────────────

/// Directional bias of a detected signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Long,
    Short,
    Neutral,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Long => f.write_str("Long"),
            Direction::Short => f.write_str("Short"),
            Direction::Neutral => f.write_str("Neutral"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SignalType
// ─────────────────────────────────────────────────────────────────────────────

/// Taxonomy of supported signal categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalType {
    WhaleFlow,
    SmartMoney,
    Momentum,
}

impl std::fmt::Display for SignalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalType::WhaleFlow => f.write_str("WhaleFlow"),
            SignalType::SmartMoney => f.write_str("SmartMoney"),
            SignalType::Momentum => f.write_str("Momentum"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FeatureVector
// ─────────────────────────────────────────────────────────────────────────────

/// Structured numeric features that drove a signal — attached for explainability.
///
/// All values are dimensionless ratios or normalised scores (no USD, no raw ticks).
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureVector {
    /// Raw-unit volume sum over the short window (≈ 30 s).
    pub volume_short: f64,
    /// Raw-unit volume sum over the long window (`momentum_window_secs`).
    pub volume_long: f64,
    /// (last_price − first_price) / first_price over the long window; 0.0 if < 2 samples.
    pub price_velocity: f64,
    /// (last_liq − first_liq) / first_liq over the long window; 0.0 if < 2 samples.
    pub liquidity_delta_pct: f64,
    /// Normalised whale-activity score in [0.0, 1.0].
    pub whale_activity_score: f64,
    /// Normalised smart-money score in [0.0, 1.0].
    pub smart_money_score: f64,
    /// Number of market-data points available; used to bound confidence.
    pub data_points: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// SignalEvent
// ─────────────────────────────────────────────────────────────────────────────

/// Output of the signal engine: a single, fully-explained signal.
///
/// No trading or execution logic may be attached to this type.
#[derive(Clone, Debug)]
pub struct SignalEvent {
    /// Deterministic FNV-1a hash of (pool, signal_type, timestamp).
    pub signal_id: u64,
    /// UNIX timestamp in microseconds when the signal was produced.
    pub timestamp_micros: u64,
    /// Address of the liquidity pool this signal refers to.
    pub pool_address: Pubkey,
    /// Signal category.
    pub signal_type: SignalType,
    /// Normalised signal magnitude in [0.0, 1.0].
    pub strength: f64,
    /// Normalised confidence in the strength estimate, in [0.0, 1.0].
    pub confidence: f64,
    /// Directional bias.
    pub direction: Direction,
    /// Rolling window length (seconds) used to compute this signal.
    pub timeframe_secs: u64,
    /// Structured feature snapshot that produced this signal.
    pub feature_vector: FeatureVector,
    /// Deterministic human-readable reasoning trace.
    pub explanation: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// WhaleEvent
// ─────────────────────────────────────────────────────────────────────────────

/// A large-wallet swap observation injected by the caller (e.g. a whale-tracker).
///
/// The signal engine does not derive this from raw on-chain data itself; it
/// consumes externally-classified events.  No wallet addresses are stored —
/// only derived metrics — to avoid hardcoded wallet lists.
#[derive(Clone, Debug)]
pub struct WhaleEvent {
    /// UNIX timestamp in microseconds when the swap was observed.
    pub timestamp_micros: u64,
    /// Pool on which the swap occurred.
    pub pool_address: Pubkey,
    /// Approximate USD size of the swap (caller-provided estimation).
    pub swap_amount_usd: f64,
    /// Directional bias of the swap.
    pub direction: Direction,
    /// Caller-supplied wallet profitability score in [0.0, 1.0].
    pub profitability_score: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// SignalInput
// ─────────────────────────────────────────────────────────────────────────────

/// Discriminated union of all event types the signal engine can process.
#[derive(Clone, Debug)]
pub enum SignalInput {
    Market(MarketEvent),
    Whale(WhaleEvent),
}

impl SignalInput {
    /// Returns the pool address carried by this input, if one is available.
    pub fn pool_address(&self) -> Option<Pubkey> {
        match self {
            SignalInput::Market(me) => market_event_pool(me),
            SignalInput::Whale(we) => Some(we.pool_address),
        }
    }
}

/// Extracts the pool address from a `MarketEvent`, if present.
pub(crate) fn market_event_pool(event: &MarketEvent) -> Option<Pubkey> {
    use common::{PoolUpdate, SwapEvent, TickUpdate};
    match event {
        MarketEvent::PoolUpdate(PoolUpdate { pool, .. }) => *pool,
        MarketEvent::SwapEvent(SwapEvent { pool, .. }) => Some(*pool),
        MarketEvent::TickUpdate(TickUpdate { pool, .. }) => Some(*pool),
        MarketEvent::PriceUpdate(_) => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Deterministic FNV-1a u64 hash used for `signal_id` generation.
pub(crate) fn fnv1a(pool: Pubkey, signal_type_tag: u8, timestamp_micros: u64) -> u64 {
    const OFFSET: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;

    let mut h = OFFSET;
    for &b in pool.as_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(PRIME);
    }
    h ^= u64::from(signal_type_tag);
    h = h.wrapping_mul(PRIME);
    for &b in timestamp_micros.to_le_bytes().iter() {
        h ^= u64::from(b);
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// Formats a `Pubkey` as a short `"aabb..yyzz"` hex string for human-readable traces.
pub(crate) fn short_addr(pool: Pubkey) -> String {
    let b = pool.to_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}..{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[30], b[31]
    )
}

/// Returns the current UNIX time in microseconds.
pub(crate) fn unix_micros() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Pubkey;

    #[test]
    fn fnv1a_is_deterministic() {
        let p = Pubkey::new([1; 32]);
        assert_eq!(fnv1a(p, 0, 1_000), fnv1a(p, 0, 1_000));
    }

    #[test]
    fn fnv1a_differs_by_input() {
        let p = Pubkey::new([1; 32]);
        assert_ne!(fnv1a(p, 0, 1_000), fnv1a(p, 1, 1_000));
        assert_ne!(fnv1a(p, 0, 1_000), fnv1a(p, 0, 2_000));
    }

    #[test]
    fn short_addr_has_expected_format() {
        let p = Pubkey::new([0xde, 0xad, 0xbe, 0xef, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xca, 0xfe]);
        let s = short_addr(p);
        assert!(s.starts_with("deadbeef.."), "got: {s}");
        assert!(s.ends_with("cafe"), "got: {s}");
    }

    #[test]
    fn signal_input_pool_address_market_swap() {
        let pool = Pubkey::new([7; 32]);
        let evt = MarketEvent::SwapEvent(common::SwapEvent {
            pool,
            input_mint: Pubkey::new([1; 32]),
            output_mint: Pubkey::new([2; 32]),
            amount_in: 100,
            amount_out: 99,
        });
        assert_eq!(SignalInput::Market(evt).pool_address(), Some(pool));
    }

    #[test]
    fn direction_display() {
        assert_eq!(Direction::Long.to_string(), "Long");
        assert_eq!(Direction::Short.to_string(), "Short");
        assert_eq!(Direction::Neutral.to_string(), "Neutral");
    }
}
