//! Input subscribers — wallet, market, arb, microstructure.

use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Sender;
use tracing::info;

use crate::config::AlphaConfig;
use crate::fusion::FusionEngine;
use crate::types::{
    ArbSignal, LiquiditySnapshot, MarketSignal, MicrostructureEvent, WalletScore, unix_ms,
};

const SOL: &str = "So11111111111111111111111111111111111111112";

pub enum InputEvent {
    Wallet(WalletScore),
    Market(MarketSignal),
    Micro(MicrostructureEvent),
    Arb(ArbSignal),
    Liquidity(LiquiditySnapshot),
}

pub fn spawn_mock_inputs(config: Arc<AlphaConfig>, event_tx: Sender<InputEvent>) {
    let interval = config.tick_interval_ms;
    std::thread::spawn(move || {
        let mut tick: u64 = 0;
        info!("alpha mock inputs started ({interval}ms)");
        loop {
            tick += 1;
            let ts = unix_ms();
            let _ = event_tx.send(InputEvent::Wallet(WalletScore {
                wallet: format!("whale_{}", tick % 3),
                score: 0.7 + (tick % 5) as f64 * 0.04,
                confidence: 0.75,
                token: SOL.into(),
                timestamp: ts,
            }));
            let _ = event_tx.send(InputEvent::Market(MarketSignal {
                token: SOL.into(),
                momentum: 0.4 + (tick % 7) as f64 * 0.05,
                volume_spike: 1.5 + (tick % 4) as f64 * 0.5,
                price_change: 0.01 + (tick % 3) as f64 * 0.005,
                timestamp: ts,
            }));
            let _ = event_tx.send(InputEvent::Liquidity(LiquiditySnapshot {
                token: SOL.into(),
                liquidity_usd: 50_000.0,
                spread_bps: 25.0,
                timestamp: ts,
            }));
            if tick % 3 == 0 {
                let _ = event_tx.send(InputEvent::Arb(ArbSignal {
                    token_pair: format!("{SOL}/USDC"),
                    spread_pct: 0.004 + (tick % 2) as f64 * 0.002,
                    confidence: 0.8,
                }));
            }
            std::thread::sleep(Duration::from_millis(interval));
        }
    });
}

pub fn spawn_input_consumer(fusion: Arc<FusionEngine>, event_rx: crossbeam_channel::Receiver<InputEvent>) {
    std::thread::spawn(move || {
        while let Ok(ev) = event_rx.recv() {
            match ev {
                InputEvent::Wallet(w) => fusion.on_wallet(&w),
                InputEvent::Market(m) => fusion.on_market(&m),
                InputEvent::Micro(m) => fusion.on_microstructure(&m),
                InputEvent::Arb(a) => fusion.on_arb(&a),
                InputEvent::Liquidity(l) => fusion.on_liquidity(&l),
            }
        }
    });
}

pub fn derive_market_from_swap(token: &str, amount_sol: f64, prev_avg: f64) -> MarketSignal {
    let volume_spike = if prev_avg > 0.0 {
        amount_sol / prev_avg
    } else {
        1.0
    };
    MarketSignal {
        token: token.into(),
        momentum: volume_spike.min(1.0) * 0.5,
        volume_spike,
        price_change: 0.01,
        timestamp: unix_ms(),
    }
}
