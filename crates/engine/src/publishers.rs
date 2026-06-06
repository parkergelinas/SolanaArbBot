//! Strategy publishers — bridge detection loops to the shared [`SignalBus`].

use std::sync::Arc;

use config::{MomentumConfig, SystemConfig};
use crossbeam_channel::{Receiver, Sender};
use pricing::TokenQualityFilter;
use signals::{
    copy_signal_channel, spawn_copy_trader, spawn_liquidation_hunter, spawn_quote_arb_publisher,
    CopySignal, CopyTraderState, LiquidationStore, QuoteArbSignal, RecentSalesTracker,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::arbitrage::{ArbitrageEngine, DexVenue, PoolQuote};
use crate::dispatcher::{Signal, SignalBus, SignalPayload, StrategyMode};

/// Output of publisher startup — optional copy channel for whale watcher wiring.
pub struct PublisherHandles {
    pub tasks: Vec<(&'static str, tokio::task::JoinHandle<()>)>,
    pub copy_tx: Option<Sender<CopySignal>>,
}

/// Spawns all enabled strategy publishers.
pub fn spawn_strategy_publishers(
    config: Arc<SystemConfig>,
    bus: SignalBus,
    paper_mode: bool,
    shutdown: CancellationToken,
) -> PublisherHandles {
    let mut tasks = Vec::new();
    let mut copy_tx = None;
    let limiter = Arc::new(ratelimit::RateLimiter::free_tier_defaults());

    let arb_on = paper_mode || config.strategy.arb;
    if arb_on {
        tasks.push((
            "arb_publisher",
            spawn_arbitrage_publisher(Arc::clone(&config), bus.clone(), shutdown.clone()),
        ));
    } else {
        info!(subsystem = "arb_publisher", "disabled via strategy.arb");
    }

    if (paper_mode || config.strategy.quote_arb) && config.quote_arb.enabled {
        let quality = TokenQualityFilter::new();
        let cfg = Arc::new(config.quote_arb.clone());
        let ds = Arc::new(config.data_sources.clone());
        let bus_clone = bus.clone();
        tasks.push((
            "quote_arb_publisher",
            spawn_quote_arb_publisher(cfg, ds, quality, shutdown.clone(), move |sig| {
                bus_clone.publish(quote_arb_to_signal(&sig));
            }),
        ));
    } else if config.quote_arb.enabled {
        info!(subsystem = "quote_arb_publisher", "disabled via strategy.quote_arb");
    }

    if config.strategy.sniper && config.sniper.enabled {
        info!(subsystem = "sniper_publisher", "sniper module not wired — skipping");
    }

    if config.strategy.whale_copy && config.copy_trading.enabled {
        let (tx, rx) = copy_signal_channel(256);
        let (exec_tx, exec_rx) = copy_signal_channel(256);
        copy_tx = Some(tx);

        tasks.push((
            "copy_fanout",
            spawn_copy_fanout(rx, exec_tx, bus.clone()),
        ));

        spawn_copy_trader(
            exec_rx,
            Arc::new(config.copy_trading.clone()),
            Arc::new(config.features.clone()),
            Arc::new(config.data_sources.clone()),
            CopyTraderState::new(RecentSalesTracker::new()),
            None,
        );
    }

    if config.strategy.liquidation && config.liquidation.enabled {
        let store = LiquidationStore::new();
        spawn_liquidation_hunter(
            Arc::new(config.liquidation.clone()),
            Arc::new(config.features.clone()),
            Arc::new(config.data_sources.clone()),
            Arc::clone(&limiter),
            store.clone(),
        );
        tasks.push((
            "liquidation_publisher",
            spawn_liquidation_publisher(store, Arc::clone(&config), bus.clone(), shutdown.clone()),
        ));
    }

    if config.strategy.momentum && config.momentum.enabled {
        tasks.push((
            "momentum_publisher",
            spawn_momentum_bus_publisher(
                Arc::new(config.momentum.clone()),
                bus.clone(),
                shutdown.clone(),
            ),
        ));
    }

    PublisherHandles { tasks, copy_tx }
}

fn spawn_copy_fanout(
    rx: Receiver<CopySignal>,
    exec_tx: Sender<CopySignal>,
    bus: SignalBus,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        while let Ok(signal) = rx.recv() {
            bus.publish(copy_to_signal(&signal));
            let _ = exec_tx.try_send(signal);
        }
    })
}

fn quote_arb_to_signal(sig: &QuoteArbSignal) -> Signal {
    Signal {
        strategy: StrategyMode::QuoteArb,
        signal_id: format!(
            "qarb-{}-{}",
            sig.pair_label.replace('→', "-"),
            sig.detected_at_ms
        ),
        notional_usd: sig.expected_pnl_usd.abs().max(1.0),
        expected_pnl_usd: sig.expected_pnl_usd,
        payload: SignalPayload::QuoteArb {
            pair_label: sig.pair_label.clone(),
            input_mint: sig.input_mint.clone(),
            output_mint: sig.output_mint.clone(),
            route_divergence_bps: sig.route_divergence_bps,
            price_dislocation_bps: sig.price_dislocation_bps,
            edge_bps: sig.edge_bps,
            expected_pnl_usd: sig.expected_pnl_usd,
        },
    }
}

fn copy_to_signal(copy: &CopySignal) -> Signal {
    Signal {
        strategy: StrategyMode::CopyTrading,
        signal_id: copy.signature.clone(),
        notional_usd: copy.amount_usd,
        expected_pnl_usd: 0.0,
        payload: SignalPayload::CopyTrading {
            wallet: copy.wallet.clone(),
            token_out: copy.token_out.clone(),
            amount_usd: copy.amount_usd,
        },
    }
}

fn spawn_arbitrage_publisher(
    config: Arc<SystemConfig>,
    bus: SignalBus,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let sol_price_usd: f64 = std::env::var("SOLANA_ARB_SOL_PRICE_USD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(150.0);

    let mut engine = ArbitrageEngine::new(
        config.arbitrage.clone(),
        config.execution.clone(),
        config.features.clone(),
        sol_price_usd,
    );
    engine.upsert_pool(
        "ray-sol-usdc",
        PoolQuote {
            venue: DexVenue::RaydiumAmmV4,
            token_a: "SOL".into(),
            token_b: "USDC".into(),
            mid_price: 150.0,
            reserve_a: 50_000.0,
            reserve_b: 7_500_000.0,
            liquidity_usd: 80_000.0,
            fee_bps: 25,
        },
    );
    engine.upsert_pool(
        "orca-sol-usdc",
        PoolQuote {
            venue: DexVenue::OrcaWhirlpool,
            token_a: "SOL".into(),
            token_b: "USDC".into(),
            mid_price: 151.5,
            reserve_a: 50_000.0,
            reserve_b: 7_575_000.0,
            liquidity_usd: 80_000.0,
            fee_bps: 20,
        },
    );

    let shared = Arc::new(Mutex::new(engine));

    tokio::spawn(async move {
        let interval = std::time::Duration::from_millis(100);
        loop {
            if shutdown.is_cancelled() {
                return;
            }

            let opps = {
                let eng = shared.lock().await;
                eng.detect_opportunities()
            };

            for opp in opps {
                bus.publish(Signal {
                    strategy: StrategyMode::Arbitrage,
                    signal_id: format!("arb-{}-{}", opp.token_pair, opp.spread_bps as u64),
                    notional_usd: config.execution.simulation_initial_amount_usd,
                    expected_pnl_usd: opp.net_profit_usd,
                    payload: SignalPayload::Arbitrage {
                        pair: opp.token_pair,
                        spread_bps: opp.spread_bps,
                        net_profit_usd: opp.net_profit_usd,
                    },
                });
            }

            tokio::time::sleep(interval).await;
        }
    })
}

fn spawn_liquidation_publisher(
    store: LiquidationStore,
    config: Arc<SystemConfig>,
    bus: SignalBus,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval = std::time::Duration::from_millis(config.liquidation.scan_interval_ms);
        loop {
            if shutdown.is_cancelled() {
                return;
            }

            for pos in store.positions_by_tier(signals::liquidation::HealthTier::Critical) {
                let plan = signals::liquidation::plan_liquidation(&pos, &config.liquidation);
                bus.publish(Signal {
                    strategy: StrategyMode::Liquidation,
                    signal_id: format!("liq-{}", pos.obligation_pubkey),
                    notional_usd: plan.repay_amount_usd,
                    expected_pnl_usd: plan.expected_bonus_usd,
                    payload: SignalPayload::Liquidation {
                        obligation: pos.obligation_pubkey.clone(),
                        health: pos.health,
                        expected_bonus_usd: plan.expected_bonus_usd,
                    },
                });
            }

            tokio::time::sleep(interval).await;
        }
    })
}

fn spawn_momentum_bus_publisher(
    cfg: Arc<MomentumConfig>,
    bus: SignalBus,
    shutdown: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut seq: u64 = 0;
        loop {
            if shutdown.is_cancelled() {
                return;
            }

            let strength = 0.55 + (seq % 10) as f64 * 0.03;
            let confidence = cfg.min_confidence + (seq % 5) as f64 * 0.02;
            if confidence >= cfg.min_confidence {
                bus.publish(Signal {
                    strategy: StrategyMode::Momentum,
                    signal_id: format!("mom-{seq}"),
                    notional_usd: cfg.position_usd,
                    expected_pnl_usd: 0.0,
                    payload: SignalPayload::Momentum {
                        pool: format!("pool_{seq:04}"),
                        strength,
                        confidence,
                    },
                });
            }

            seq = seq.wrapping_add(1);
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        }
    })
}
