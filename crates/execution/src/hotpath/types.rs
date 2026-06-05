//! Fixed-size, `Copy`-friendly types for the hot loop.
//!
//! No `String`, no heap allocation, no serde in this module.

/// Index into the preallocated pool table.
pub type PoolIdx = u16;

/// Maximum pools supported (must match config upper bound).
pub const MAX_POOLS: usize = 128;

/// Maximum hops in a precomputed route.
pub const MAX_ROUTE_HOPS: usize = 2;

/// Fixed-point scale: prices stored as `price_fp / PRICE_SCALE`.
pub const PRICE_SCALE: u64 = 1_000_000_000;

/// Compact market tick — the only input to the hot loop from ingestion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarketTick {
    pub pool_idx: PoolIdx,
    pub slot: u64,
    /// Mid price in fixed-point (quote per base × PRICE_SCALE).
    pub price_fp: u64,
    /// Base reserve (lamports or token atoms, pool-native units).
    pub reserve_a: u64,
    /// Quote reserve.
    pub reserve_b: u64,
    /// Volume delta since last tick (USD × 100 for precision without floats).
    pub volume_delta_usd_x100: u64,
}

/// DEX venue identifier — precomputed routes reference this, not Jupiter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Venue {
    RaydiumAmm = 0,
    OrcaWhirlpool = 1,
}

/// Inline signal output — produced synchronously from pool features.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotSignal {
    pub pool_idx: PoolIdx,
    pub slot: u64,
    /// Expected edge in basis points (integer, no float in hot path).
    pub edge_bps: u32,
    /// Signal strength 0–1000.
    pub strength_x1000: u32,
    /// 1 = long, 2 = short.
    pub direction: u8,
    /// Precomputed route index into `PrecomputeTable::routes`.
    pub route_idx: u8,
}

/// Risk gate verdict — `Copy`, no allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiskVerdict {
    Approved,
    RejectedEdge,
    RejectedSlippage,
    RejectedLiquidity,
    RejectedCooldown,
    RejectedExposure,
    RejectedTradingDisabled,
}

impl RiskVerdict {
    #[must_use]
    pub const fn is_approved(self) -> bool {
        matches!(self, Self::Approved)
    }
}

/// Execution intent queued to the cold-path I/O thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionIntent {
    pub pool_idx: PoolIdx,
    pub route_idx: u8,
    pub slot: u64,
    pub size_lamports: u64,
    /// Minimum acceptable output (slippage guard).
    pub min_out: u64,
    pub edge_bps: u32,
    pub use_jito: bool,
    pub tip_lamports: u64,
}

/// Outcome of processing one market tick through the hot loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickOutcome {
    NoSignal,
    SignalRejected(RiskVerdict),
    Queued(ExecutionIntent),
}

/// Latency sample captured outside the hot loop (cold path only).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LatencySample {
    pub decision_us: u64,
    pub slot: u64,
}
