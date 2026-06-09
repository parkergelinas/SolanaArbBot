//! Adapter: data-layer enriched swaps → `LiveSignal`.

use data_layer::types::{EnrichedSwapEvent, SwapEvent};

use crate::classify::strategy_tag_for;
use crate::types::{
    AlertType, LiveSignal, SignalKind, SignalSourceMeta, SCHEMA_VERSION,
};

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Map enriched data-layer output into the shared live signal schema.
pub fn from_enriched(ev: &EnrichedSwapEvent) -> LiveSignal {
    from_swap_with_meta(
        &ev.swap,
        ev.slot,
        &ev.token_symbol,
        ev.wallet_label.as_deref(),
        ev.notional_usd,
    )
}

/// Map a bare data-layer swap into `LiveSignal`.
pub fn from_swap(swap: &SwapEvent, slot: u64) -> LiveSignal {
    from_swap_with_meta(swap, slot, &short_mint(&swap.token), None, swap.amount_sol)
}

fn from_swap_with_meta(
    swap: &SwapEvent,
    slot: u64,
    token_symbol: &str,
    wallet_label: Option<&str>,
    notional_usd: f64,
) -> LiveSignal {
    let alert_type = match wallet_label {
        Some("whale") => AlertType::WhaleFlow,
        Some("smart") => AlertType::SmartMoney,
        _ if swap.amount_sol >= 10.0 => AlertType::WhaleFlow,
        _ => AlertType::Momentum,
    };
    let strength = (swap.amount_sol / 100.0).clamp(0.05, 1.0);
    let confidence = match wallet_label {
        Some("whale") => 0.92,
        Some("smart") => 0.88,
        Some(_) => 0.75,
        None => 0.65,
    };
    let strategy_tag = strategy_tag_for(alert_type, strength, confidence, Some(notional_usd));
    let (token_in, token_out) = infer_pair(&swap.token);
    let pair = format!("{}/{}", symbol_for_mint(&token_in), symbol_for_mint_or(&token_out, token_symbol));
    let price = if swap.amount_sol > 0.0 {
        notional_usd / swap.amount_sol
    } else {
        0.0
    };
    let explanation = format!(
        "Swap {:.2} SOL (~${:.0}) on {} · {} · {}",
        swap.amount_sol, notional_usd, swap.dex, token_symbol, swap.wallet
    );

    LiveSignal {
        v: SCHEMA_VERSION,
        signal_id: swap.signature.clone(),
        kind: SignalKind::Swap,
        source: SignalSourceMeta {
            layer: "data-layer".into(),
            dex: swap.dex.clone(),
            slot,
            wallet_label: wallet_label.map(str::to_owned),
        },
        pair,
        token_in,
        token_out,
        timestamp_ms: swap.timestamp,
        tx_id: swap.signature.clone(),
        price,
        size: swap.amount_sol,
        confidence,
        wallet: swap.wallet.clone(),
        strength: Some(strength),
        size_usd: Some(notional_usd),
        alert_type: Some(alert_type),
        strategy_tag: Some(strategy_tag),
        explanation: Some(explanation),
        direction: Some("Long".into()),
        dedup_key: swap.signature.clone(),
    }
}

fn infer_pair(token: &str) -> (String, String) {
    if token == SOL_MINT || token.eq_ignore_ascii_case("sol") {
        (SOL_MINT.into(), USDC_MINT.into())
    } else {
        (SOL_MINT.into(), token.to_owned())
    }
}

fn symbol_for_mint(mint: &str) -> &str {
    match mint {
        SOL_MINT => "SOL",
        USDC_MINT => "USDC",
        _ => "TOKEN",
    }
}

fn symbol_for_mint_or<'a>(mint: &str, fallback: &'a str) -> &'a str {
    match mint {
        SOL_MINT => "SOL",
        USDC_MINT => "USDC",
        _ => fallback,
    }
}

fn short_mint(mint: &str) -> String {
    if mint.len() <= 10 {
        return mint.to_string();
    }
    format!("{}…{}", &mint[..4], &mint[mint.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_layer::types::SwapEvent;

    #[test]
    fn normalizes_enriched_swap() {
        let swap = SwapEvent {
            signature: "sig123".into(),
            wallet: "whale_alpha".into(),
            token: SOL_MINT.into(),
            amount_sol: 50.0,
            dex: "raydium".into(),
            timestamp: 1_700_000_000,
        };
        let enriched = EnrichedSwapEvent {
            v: 1,
            swap,
            token_symbol: "SOL".into(),
            wallet_label: Some("whale".into()),
            notional_usd: 7_250.0,
            slot: 42,
        };

        let live = from_enriched(&enriched);
        assert_eq!(live.tx_id, "sig123");
        assert_eq!(live.alert_type, Some(AlertType::WhaleFlow));
        assert_eq!(live.source.slot, 42);
        assert!(live.confidence > 0.9);
        assert_eq!(live.dedup_key, "sig123");
    }
}
