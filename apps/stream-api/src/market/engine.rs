//! Processes swap ingress and emits derived WS messages (no locks on read path).

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crossbeam_channel::{Receiver, Sender};

use crate::contracts::{
    Candle, CandleInterval, Dex, Signal, SignalKind, SwapEvent, TokenPrice, WSMessage,
    SCHEMA_VERSION,
};
use crate::market::state::{CandleKey, FlowTracker, MarketState, PoolSnapshot};

pub struct MarketEngineHandle {
    pub state: Arc<MarketState>,
}

fn parse_amount(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(0.0)
}

fn implied_price_usd(amount_in: f64, amount_out: f64, ref_price_in: f64) -> f64 {
    if amount_out <= 0.0 {
        return ref_price_in;
    }
    (amount_in / amount_out).max(0.0) * ref_price_in
}

fn ref_price_for_mint(mint: &str) -> f64 {
    match mint {
        "So11111111111111111111111111111111111111112" => 145.0,
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" | "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => {
            1.0
        }
        _ => 1.0,
    }
}

fn bucket_open(ts_ms: u64, interval: CandleInterval) -> u64 {
    let w = interval.bucket_ms();
    (ts_ms / w) * w
}

fn update_candle(
    state: &MarketState,
    mint: &str,
    interval: CandleInterval,
    price: f64,
    volume: f64,
    ts_ms: u64,
) -> Option<Candle> {
    let key = CandleKey {
        mint: mint.to_owned(),
        interval,
    };
    let ts_open = bucket_open(ts_ms, interval);

    let mut updated: Option<Candle> = None;
    state.candles.entry(key).and_modify(|c| {
        if c.ts_open_ms != ts_open {
            *c = Candle {
                v: SCHEMA_VERSION,
                mint: mint.to_owned(),
                interval,
                open: price,
                high: price,
                low: price,
                close: price,
                volume,
                ts_open_ms: ts_open,
            };
        } else {
            if price > c.high {
                c.high = price;
            }
            if price < c.low {
                c.low = price;
            }
            c.close = price;
            c.volume += volume;
        }
        updated = Some(c.clone());
    }).or_insert_with(|| {
        let c = Candle {
            v: SCHEMA_VERSION,
            mint: mint.to_owned(),
            interval,
            open: price,
            high: price,
            low: price,
            close: price,
            volume,
            ts_open_ms: ts_open,
        };
        updated = Some(c.clone());
        c
    });

    updated
}

fn process_swap(state: &MarketState, swap: &SwapEvent, signal_seq: &AtomicU64) -> Vec<WSMessage> {
    let mut out = Vec::with_capacity(8);
    out.push(WSMessage::Swap(swap.clone()));

    let amount_in = parse_amount(&swap.amount_in);
    let amount_out = parse_amount(&swap.amount_out);
    let ref_in = ref_price_for_mint(&swap.token_in);
    let price_out = implied_price_usd(amount_in, amount_out, ref_in);

    let price = TokenPrice {
        v: SCHEMA_VERSION,
        mint: swap.token_out.clone(),
        price_usd: price_out,
        slot: swap.slot,
        timestamp_ms: swap.timestamp_ms,
    };
    state.prices.insert(swap.token_out.clone(), price.clone());
    out.push(WSMessage::TokenPrice(price));

    let dex_label = match swap.dex {
        Dex::Raydium => "raydium",
        Dex::Orca => "orca",
        Dex::Jupiter => "jupiter",
    };
    let pool_id = format!("{dex_label}:{}:{}", swap.token_in, swap.token_out);
    state.pools.insert(
        pool_id.clone(),
        PoolSnapshot {
            pool_id: pool_id.clone(),
            token_a: swap.token_in.clone(),
            token_b: swap.token_out.clone(),
            reserve_a: amount_in as u64,
            reserve_b: amount_out as u64,
            slot: swap.slot,
        },
    );

    let volume = amount_in / 1_000_000.0;
    for interval in CandleInterval::ALL {
        if let Some(candle) =
            update_candle(state, &swap.token_out, interval, price_out, volume, swap.timestamp_ms)
        {
            out.push(WSMessage::Candle(candle));
        }
    }

    let flow = state
        .flow
        .entry(swap.token_out.clone())
        .or_insert_with(FlowTracker::new);
    flow.record(true, amount_in as u64);
    let imbalance = flow.imbalance();

    if imbalance.abs() > 0.35 && amount_in > 500_000.0 {
        let id = signal_seq.fetch_add(1, Ordering::Relaxed);
        out.push(WSMessage::Signal(Signal {
            v: SCHEMA_VERSION,
            signal_id: format!("sig_{id}"),
            mint: swap.token_out.clone(),
            kind: SignalKind::Imbalance,
            strength: imbalance.abs(),
            confidence: 0.6,
            timestamp_ms: swap.timestamp_ms,
            detail: Some(format!("flow_imbalance={imbalance:.3}")),
        }));
    }

    out
}

/// Blocking consumer on a dedicated thread — keeps tokio WS tasks allocation-light.
pub fn spawn_market_engine(
    swap_rx: Receiver<SwapEvent>,
    out_tx: Sender<WSMessage>,
) -> MarketEngineHandle {
    let state = Arc::new(MarketState::new());
    let state_worker = state.clone();
    let signal_seq = AtomicU64::new(0);

    std::thread::spawn(move || {
        while let Ok(swap) = swap_rx.recv() {
            let messages = process_swap(&state_worker, &swap, &signal_seq);
            for msg in messages {
                let _ = out_tx.send(msg);
            }
        }
    });

    MarketEngineHandle { state }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::Dex;

    #[test]
    fn swap_produces_price_and_candles() {
        let state = MarketState::new();
        let seq = AtomicU64::new(0);
        let swap = SwapEvent::new_v1(
            "s",
            Dex::Orca,
            "So11111111111111111111111111111111111111112",
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "1000000000",
            "145000000",
            "w",
            1,
            1_700_000_000_000,
        );
        let msgs = process_swap(&state, &swap, &seq);
        assert!(msgs.iter().any(|m| matches!(m, WSMessage::TokenPrice(_))));
        assert!(state.candles.len() >= 3);
    }
}
