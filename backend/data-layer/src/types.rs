//! Canonical internal schemas — aligned with `shared/contracts/intelligence/v1.ts`.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

/// Raw blockchain update from Yellowstone or mock adapter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawUpdate {
    pub slot: u64,
    pub signature: String,
    pub logs: Vec<String>,
    pub accounts: Vec<String>,
    pub timestamp: u64,
}

/// Unified normalized swap (canonical downstream schema).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SwapEvent {
    pub signature: String,
    pub wallet: String,
    pub token: String,
    pub amount_sol: f64,
    pub dex: String,
    pub timestamp: u64,
}

/// Enriched swap with metadata attached.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnrichedSwapEvent {
    pub v: u32,
    #[serde(flatten)]
    pub swap: SwapEvent,
    pub token_symbol: String,
    pub wallet_label: Option<String>,
    pub notional_usd: f64,
    pub slot: u64,
}

/// Parsed swap leg before normalization.
#[derive(Clone, Debug)]
pub struct ParsedSwapLeg {
    pub wallet: String,
    pub token: String,
    pub amount_sol: f64,
    pub dex: String,
}

/// Routed pipeline event for downstream consumers.
#[derive(Clone, Debug)]
pub enum PipelineEvent {
    EnrichedSwap(EnrichedSwapEvent),
}

impl PipelineEvent {
    pub fn enriched(&self) -> &EnrichedSwapEvent {
        match self {
            Self::EnrichedSwap(e) => e,
        }
    }
}
