//! Alpha engine — fusion, scoring, queue, execution emission.

use std::sync::Arc;

use crossbeam_channel::unbounded;
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use alpha_engine::{
    config::AlphaConfig,
    emitter::SignalEmitter,
    fusion::FusionEngine,
    inputs::{spawn_mock_inputs, InputEvent},
    pipeline::AlphaPipeline,
    types::{token_from_pair, TradeSignal},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Arc::new(AlphaConfig::from_env());
    info!(
        min_score = config.min_alpha_score,
        tick_ms = config.tick_interval_ms,
        "alpha-engine starting"
    );

    let fusion = Arc::new(FusionEngine::new(config.clone()));

    let (trade_tx, mut trade_rx) = mpsc::unbounded_channel::<TradeSignal>();
    let emitter = SignalEmitter::new(trade_tx);
    let pipeline = Arc::new(AlphaPipeline::new(config.clone(), fusion.clone(), emitter));

    let (event_tx, event_rx) = unbounded::<InputEvent>();

    if config.mock_mode {
        spawn_mock_inputs(config.clone(), event_tx);
    }

    let pipeline_worker = pipeline.clone();
    let fusion_worker = fusion.clone();
    std::thread::spawn(move || {
        while let Ok(ev) = event_rx.recv() {
            let (token, wallet) = match &ev {
                InputEvent::Wallet(w) => {
                    fusion_worker.on_wallet(w);
                    (w.token.clone(), w.wallet.clone())
                }
                InputEvent::Market(m) => {
                    fusion_worker.on_market(m);
                    (m.token.clone(), "market".into())
                }
                InputEvent::Micro(m) => {
                    fusion_worker.on_microstructure(m);
                    (m.token.clone(), "micro".into())
                }
                InputEvent::Arb(a) => {
                    fusion_worker.on_arb(a);
                    (token_from_pair(&a.token_pair), "arb_engine".into())
                }
                InputEvent::Liquidity(l) => {
                    fusion_worker.on_liquidity(l);
                    (l.token.clone(), "liquidity".into())
                }
            };
            pipeline_worker.on_token_updated(&token, &wallet);
        }
    });

    let pipeline_drain = pipeline.clone();
    let tick_ms = config.tick_interval_ms;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(tick_ms));
        loop {
            interval.tick().await;
            let n = pipeline_drain.drain_tick();
            if n > 0 {
                info!(
                    emitted = n,
                    depth = pipeline_drain.queue_depth(),
                    "queue drain tick"
                );
            }
        }
    });

    tokio::spawn(async move {
        while trade_rx.recv().await.is_some() {}
    });

    info!("alpha-engine running");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
