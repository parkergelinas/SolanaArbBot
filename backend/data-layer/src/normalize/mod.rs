//! Normalization layer — parsed legs → canonical `SwapEvent`.

use crate::types::{ParsedSwapLeg, RawUpdate, SwapEvent};

pub fn normalize(raw: &RawUpdate, leg: &ParsedSwapLeg) -> SwapEvent {
    SwapEvent {
        signature: raw.signature.clone(),
        wallet: leg.wallet.clone(),
        token: leg.token.clone(),
        amount_sol: leg.amount_sol,
        dex: leg.dex.clone(),
        timestamp: raw.timestamp,
    }
}
