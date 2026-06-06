//! Autonomous scalping + DEX-to-DEX loop with risk enforcement.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::{MarketEvent, Pubkey};
use config::SystemConfig;
use decoder::{DexDecoder, DexType, PoolState};
use execution::ExecutionSimulator;
use graph::{Edge, MarketGraph};
use orchestrator::{
    ExecutionApprovalRequest, Orchestrator,
    state::{HealthStatus, RiskState},
};
use pricing::PricingEngine;
use risk::RiskEngine;
use risk_engine::{evaluate_limits, RiskAction, RiskConfig as EnforcerConfig, RiskState as EnforcerState};
use routing::{Route, Router};
use scalper::ScalpEngine;
use signals::{Direction, SignalEngine, SignalEvent, SignalInput, SignalType, WhaleEvent};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::ingestion::{default_pools, tick_events, ExternalIngestionBuffer};
use crate::snapshot::RuntimeSnapshot;
use crate::strategy::{IngestionMode, StrategyController};
use crate::trade_emit::{new_trade_id, TradeEmit, TradeStage};

const TICK_MS: u64 = 500;
const NETWORK_COST_USD: f64 = 0.05;
const SLIPPAGE_HAIRCUT: f64 = 0.20;

pub struct AutonomousCallbacks {
    pub on_signal: Option<Arc<dyn Fn(SignalEvent) + Send + Sync>>,
    pub on_trade: Option<Arc<dyn Fn(TradeEmit) + Send + Sync>>,
}

fn emit_trade(callbacks: &AutonomousCallbacks, event: TradeEmit) {
    if let Some(ref cb) = callbacks.on_trade {
        cb(event);
    }
}

fn signal_side_str(direction: Direction) -> &'static str {
    match direction {
        Direction::Long => "long",
        Direction::Short => "short",
        Direction::Neutral => "buy",
    }
}

fn now_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_micros() as u64)
}

fn pool_tvl(pair_byte: u8) -> f64 {
    800_000.0 + f64::from(pair_byte) * 100_000.0
}

fn theoretical_profit(route: &Route, trade_size: f64) -> f64 {
    let product = route.path.iter().fold(1.0_f64, |acc, edge| {
        acc * edge.price * (1.0 - edge.fee_bps as f64 / 10_000.0)
    });
    trade_size * (product - 1.0)
}

fn rebuild_graph(pools: &HashMap<Pubkey, PoolState>) -> MarketGraph {
    let mut graph = MarketGraph::new();
    for pool in pools.values() {
        let liq = pool.liquidity as f64;
        if let Some((a, b)) = pool.reserves {
            if a > 0 && b > 0 {
                let _ = graph.add_edge(Edge::new(
                    pool.token_a.clone(),
                    pool.token_b.clone(),
                    b as f64 / a as f64,
                    liq,
                    25,
                ));
                let _ = graph.add_edge(Edge::new(
                    pool.token_b.clone(),
                    pool.token_a.clone(),
                    a as f64 / b as f64,
                    liq,
                    25,
                ));
            }
        }
    }
    graph
}

/// Spawns the autonomous trading runtime with strategy policy control.
pub fn spawn_autonomous_runtime(
    config: Arc<RwLock<SystemConfig>>,
    strategy: Arc<RwLock<StrategyController>>,
    external: Arc<RwLock<ExternalIngestionBuffer>>,
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
    cancel: CancellationToken,
    callbacks: AutonomousCallbacks,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_loop(config, strategy, external, snapshot, cancel, callbacks).await {
            warn!(error = %e, "autonomous runtime exited with error");
        }
    })
}

fn sync_snapshot_strategy(snap: &mut RuntimeSnapshot, plan: &crate::strategy::CyclePlan) {
    snap.mode = plan.runtime_mode.as_str().to_owned();
    snap.runtime_mode = plan.runtime_mode.as_str().to_owned();
    snap.ingestion_mode = plan.ingestion.mode.as_str().to_owned();
    snap.active_strategies = plan.active_strategies.clone();
    snap.scalp_enabled = plan.trade.scalp_enabled;
    snap.arb_enabled = plan.trade.arb_enabled;
}

fn record_signal_consumed(snap: &mut RuntimeSnapshot, signal: &SignalEvent) {
    snap.signals_consumed += 1;
    match signal.signal_type {
        SignalType::WhaleFlow | SignalType::SmartMoney => snap.whale_signals_consumed += 1,
        SignalType::Momentum => snap.momentum_signals_consumed += 1,
    }
}

async fn run_loop(
    config: Arc<RwLock<SystemConfig>>,
    strategy: Arc<RwLock<StrategyController>>,
    external: Arc<RwLock<ExternalIngestionBuffer>>,
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
    cancel: CancellationToken,
    callbacks: AutonomousCallbacks,
) -> Result<(), String> {
    let cfg_snapshot = config.read().await.clone();
    let (orch, state_rx) = Orchestrator::new(Arc::new(cfg_snapshot.clone()));
    orch.enable_trading("autonomous runtime started")
        .map_err(|e| format!("{e}"))?;

    let (signal_engine, _rx) = SignalEngine::new(cfg_snapshot.signal_engine.clone());
    let scalp_engine = ScalpEngine::new(Arc::new(cfg_snapshot.clone()));
    let decoder = DexDecoder::new();
    let pricing = PricingEngine::new(decoder);
    let risk = RiskEngine::new();

    let mut pool_states: HashMap<Pubkey, PoolState> = HashMap::new();
    let mut last_arb_ts: u64 = 0;
    let arb_cooldown_us = cfg_snapshot.scalper.trade_cooldown_secs * 1_000_000;
    let trade_size = cfg_snapshot.execution.simulation_initial_amount_usd;
    let min_profit = cfg_snapshot.execution.min_profit_threshold_usd;
    let max_loss = cfg_snapshot.execution.max_loss_per_trade_usd;
    let capital_usd = cfg_snapshot.portfolio.capital_usd;

    let mut enforcer_state = EnforcerState::new(capital_usd);
    let enforcer_cfg = EnforcerConfig {
        capital_usd,
        daily_max_loss_pct: cfg_snapshot.risk.daily_loss_limit_pct,
        monthly_max_loss_pct: cfg_snapshot.risk.monthly_loss_limit_pct,
        total_max_loss_pct: cfg_snapshot.risk.total_loss_halt_pct,
        max_drawdown_pct: cfg_snapshot.risk.max_drawdown_pct,
        daily_pause: cfg_snapshot.risk.daily_pause(),
    };

    {
        let plan = strategy.read().await.plan_cycle(&cfg_snapshot);
        let mut snap = snapshot.write().await;
        snap.running = true;
        sync_snapshot_strategy(&mut snap, &plan);
        snap.peak_equity_usd = capital_usd;
        snap.current_equity_usd = capital_usd;
        snap.risk_status = "healthy".to_owned();
        snap.risk_state = "normal".to_owned();
    }

    {
        let runtime_mode = strategy
            .read()
            .await
            .plan_cycle(&cfg_snapshot)
            .runtime_mode
            .as_str()
            .to_owned();
        info!(runtime_mode = %runtime_mode, "autonomous runtime started");
    }

    let mut step: u64 = 0;
    let t0 = now_micros();

    loop {
        if cancel.is_cancelled() {
            break;
        }

        let sys_state = state_rx.borrow().clone();
        if sys_state.risk_state == RiskState::Locked
            || sys_state.health_status == HealthStatus::Critical
        {
            let mut snap = snapshot.write().await;
            snap.trading_halted = true;
            snap.halt_reason = sys_state.reason.clone();
            snap.risk_state = "locked".to_owned();
            snap.risk_status = "critical".to_owned();
        }

        let halted = {
            let snap = snapshot.read().await;
            snap.trading_halted
        };

        let (plan, cfg) = {
            let cfg = config.read().await.clone();
            let plan = strategy.read().await.plan_cycle(&cfg);
            (plan, cfg)
        };

        {
            let mut snap = snapshot.write().await;
            sync_snapshot_strategy(&mut snap, &plan);
        }

        if !halted && plan.trade.trading_enabled {
            let ts = t0 + step * TICK_MS * 1_000;
            let market_events: Vec<(u64, MarketEvent)> = match plan.ingestion.mode {
                IngestionMode::Synthetic => tick_events(step, ts),
                IngestionMode::SignalBus | IngestionMode::Intelligence => {
                    external.write().await.drain_market()
                }
                IngestionMode::None => Vec::new(),
            };

            for (evt_ts, market_evt) in market_events {
                {
                    let mut snap = snapshot.write().await;
                    snap.events_processed += 1;
                }

                // ── Scalping path ─────────────────────────────────────────
                if !plan.trade.scalp_enabled {
                    continue;
                }

                let mut signals = signal_engine.process_at(
                    SignalInput::Market(market_evt.clone()),
                    evt_ts,
                );
                signals.retain(|s| plan.trade.signal_eligible(s));
                signals.sort_by_key(|s| plan.trade.signal_priority(s));

                if !signals.is_empty() {
                    let mut snap = snapshot.write().await;
                    snap.signals_emitted += signals.len() as u64;
                }

                for signal in signals {
                    {
                        let mut snap = snapshot.write().await;
                        record_signal_consumed(&mut snap, &signal);
                    }
                    if let Some(ref cb) = callbacks.on_signal {
                        cb(signal.clone());
                    }

                    let pool_byte = signal.pool_address.as_bytes()[0];
                    let features = signal_engine.feature_store().compute(
                        signal.pool_address,
                        evt_ts,
                        &cfg.signal_engine,
                    );

                    let gate_req = ExecutionApprovalRequest {
                        signal_id: signal.signal_id,
                        pool_address: format!("{:?}", signal.pool_address),
                        trade_size_usd: cfg.scalper.min_position_usd,
                        estimated_cost_bps: 30.0,
                        net_edge_bps: signal.strength * 10_000.0,
                        requested_at_micros: evt_ts,
                    };
                    let gate = orch.approve_execution(&gate_req);
                    let trade_id = new_trade_id("scalp");
                    let pair = format!("{:?}", signal.pool_address);
                    let side = signal_side_str(signal.direction);
                    let size_usd = cfg.scalper.min_position_usd;

                    emit_trade(
                        &callbacks,
                        TradeEmit::paper(
                            &trade_id,
                            "scalp",
                            &pair,
                            side,
                            size_usd,
                            0.0,
                            evt_ts,
                            TradeStage::Started,
                            Some(signal.signal_id),
                        ),
                    );

                    if !gate.approved {
                        emit_trade(
                            &callbacks,
                            TradeEmit::paper(
                                &trade_id,
                                "scalp",
                                &pair,
                                side,
                                size_usd,
                                0.0,
                                evt_ts,
                                TradeStage::Rejected,
                                Some(signal.signal_id),
                            )
                            .with_reject("orchestrator gate denied"),
                        );
                        continue;
                    }

                    emit_trade(
                        &callbacks,
                        TradeEmit::paper(
                            &trade_id,
                            "scalp",
                            &pair,
                            side,
                            size_usd,
                            signal.strength * size_usd,
                            evt_ts,
                            TradeStage::Validated,
                            Some(signal.signal_id),
                        ),
                    );

                    if let Some(result) =
                        scalp_engine.evaluate(signal, features, pool_tvl(pool_byte), evt_ts)
                    {
                        let expected_pnl = result.candidate.trade_size_usd
                            * (result.pnl_bps / 10_000.0);

                        emit_trade(
                            &callbacks,
                            TradeEmit::paper(
                                &trade_id,
                                "scalp",
                                &pair,
                                side,
                                result.candidate.trade_size_usd,
                                expected_pnl,
                                evt_ts,
                                TradeStage::Quoted,
                                Some(result.candidate.signal.signal_id),
                            ),
                        );

                        if result.rejected {
                            emit_trade(
                                &callbacks,
                                TradeEmit::paper(
                                    &trade_id,
                                    "scalp",
                                    &pair,
                                    side,
                                    result.candidate.trade_size_usd,
                                    0.0,
                                    evt_ts,
                                    TradeStage::Rejected,
                                    Some(result.candidate.signal.signal_id),
                                )
                                .with_reject(
                                    result
                                        .reject_reason
                                        .unwrap_or_else(|| "filter rejected".to_owned()),
                                ),
                            );
                        } else {
                            emit_trade(
                                &callbacks,
                                TradeEmit::paper(
                                    &trade_id,
                                    "scalp",
                                    &pair,
                                    side,
                                    result.candidate.trade_size_usd,
                                    expected_pnl,
                                    evt_ts,
                                    TradeStage::Submitted,
                                    Some(result.candidate.signal.signal_id),
                                ),
                            );

                            let pnl_usd = result.candidate.trade_size_usd
                                * (result.pnl_bps / 10_000.0);
                            let fee_usd = result.candidate.trade_size_usd
                                * (result.fee_paid_bps * 2.0 / 10_000.0);
                            let net = pnl_usd - fee_usd;
                            let paper_sig = format!("paper_scalp_{trade_id}");

                            emit_trade(
                                &callbacks,
                                TradeEmit::paper(
                                    &trade_id,
                                    "scalp",
                                    &pair,
                                    side,
                                    result.candidate.trade_size_usd,
                                    net,
                                    result.executed_at_micros,
                                    TradeStage::Filled,
                                    Some(result.candidate.signal.signal_id),
                                )
                                .with_tx(paper_sig),
                            );

                            let mut snap = snapshot.write().await;
                            snap.scalp_trades += 1;
                            snap.scalp_pnl_usd += net;
                            snap.signals_traded += 1;
                            if net > 0.0 {
                                snap.winning_trades += 1;
                            }
                            enforcer_state.current_equity += net;
                            if net < 0.0 {
                                enforcer_state.cumulative_daily_loss += net;
                                snap.daily_loss_usd += net.abs();
                            }
                            snap.refresh_derived(capital_usd);
                        }
                    }
                    break;
                }

                // ── DEX-to-DEX arb path ───────────────────────────────────
                if !plan.trade.arb_enabled {
                    continue;
                }
                if let MarketEvent::PoolUpdate(ref update) = market_evt {
                    let Some(pool_key) = update.pool else { continue };
                    let dex = if pool_key.as_bytes()[1] == 0 {
                        DexType::Raydium
                    } else {
                        DexType::OrcaCLMM
                    };
                    if let Ok(pool) = decoder.decode_event(dex, &market_evt) {
                        pool_states.insert(pool_key, pool);
                    }

                    if evt_ts < last_arb_ts.saturating_add(arb_cooldown_us) {
                        continue;
                    }

                    let graph = rebuild_graph(&pool_states);
                    let router = Router::new(graph, pricing);
                    let execution = ExecutionSimulator::new(router.clone());

                    for route in router.find_arbitrage_cycles() {
                        let trade_id = new_trade_id("arb");
                        let pair = route
                            .path
                            .first()
                            .map(|e| format!("{:?}/{:?}", e.from, e.to))
                            .unwrap_or_else(|| "arb-cycle".to_owned());

                        emit_trade(
                            &callbacks,
                            TradeEmit::paper(
                                &trade_id,
                                "arb",
                                &pair,
                                "buy",
                                trade_size,
                                0.0,
                                evt_ts,
                                TradeStage::Started,
                                Some(step),
                            ),
                        );

                        let gross_edge = theoretical_profit(&route, trade_size);
                        if gross_edge <= 0.0 {
                            continue;
                        }

                        emit_trade(
                            &callbacks,
                            TradeEmit::paper(
                                &trade_id,
                                "arb",
                                &pair,
                                "buy",
                                trade_size,
                                gross_edge,
                                evt_ts,
                                TradeStage::Quoted,
                                Some(step),
                            ),
                        );

                        let Ok(sim) = execution.simulate_route(&route, trade_size) else {
                            emit_trade(
                                &callbacks,
                                TradeEmit::paper(
                                    &trade_id,
                                    "arb",
                                    &pair,
                                    "buy",
                                    trade_size,
                                    0.0,
                                    evt_ts,
                                    TradeStage::Failed,
                                    Some(step),
                                )
                                .with_reject("route simulation failed"),
                            );
                            continue;
                        };

                        if !risk.evaluate_route(&route, &sim).allowed {
                            emit_trade(
                                &callbacks,
                                TradeEmit::paper(
                                    &trade_id,
                                    "arb",
                                    &pair,
                                    "buy",
                                    trade_size,
                                    0.0,
                                    evt_ts,
                                    TradeStage::Rejected,
                                    Some(step),
                                )
                                .with_reject("risk gate denied route"),
                            );
                            continue;
                        }

                        emit_trade(
                            &callbacks,
                            TradeEmit::paper(
                                &trade_id,
                                "arb",
                                &pair,
                                "buy",
                                trade_size,
                                gross_edge,
                                evt_ts,
                                TradeStage::Validated,
                                Some(step),
                            ),
                        );

                        let gate_req = ExecutionApprovalRequest {
                            signal_id: step,
                            pool_address: "arb-cycle".to_owned(),
                            trade_size_usd: trade_size,
                            estimated_cost_bps: 50.0,
                            net_edge_bps: (gross_edge / trade_size) * 10_000.0,
                            requested_at_micros: evt_ts,
                        };
                        if !orch.approve_execution(&gate_req).approved {
                            emit_trade(
                                &callbacks,
                                TradeEmit::paper(
                                    &trade_id,
                                    "arb",
                                    &pair,
                                    "buy",
                                    trade_size,
                                    0.0,
                                    evt_ts,
                                    TradeStage::Rejected,
                                    Some(step),
                                )
                                .with_reject("orchestrator gate denied"),
                            );
                            continue;
                        }

                        let net = gross_edge * (1.0 - SLIPPAGE_HAIRCUT)
                            - trade_size * 0.005
                            - NETWORK_COST_USD;
                        if net < -max_loss {
                            emit_trade(
                                &callbacks,
                                TradeEmit::paper(
                                    &trade_id,
                                    "arb",
                                    &pair,
                                    "buy",
                                    trade_size,
                                    net,
                                    evt_ts,
                                    TradeStage::Rejected,
                                    Some(step),
                                )
                                .with_reject("arb net loss exceeds max_loss_per_trade_usd"),
                            );
                            continue;
                        }
                        if net < min_profit {
                            emit_trade(
                                &callbacks,
                                TradeEmit::paper(
                                    &trade_id,
                                    "arb",
                                    &pair,
                                    "buy",
                                    trade_size,
                                    net,
                                    evt_ts,
                                    TradeStage::Canceled,
                                    Some(step),
                                )
                                .with_reject("below min profit threshold"),
                            );
                            continue;
                        }

                        emit_trade(
                            &callbacks,
                            TradeEmit::paper(
                                &trade_id,
                                "arb",
                                &pair,
                                "buy",
                                trade_size,
                                net,
                                evt_ts,
                                TradeStage::Submitted,
                                Some(step),
                            ),
                        );

                        let paper_sig = format!("paper_arb_{trade_id}");
                        emit_trade(
                            &callbacks,
                            TradeEmit::paper(
                                &trade_id,
                                "arb",
                                &pair,
                                "buy",
                                trade_size,
                                net,
                                evt_ts,
                                TradeStage::Filled,
                                Some(step),
                            )
                            .with_tx(paper_sig),
                        );

                        last_arb_ts = evt_ts;
                        let mut snap = snapshot.write().await;
                        snap.arb_trades += 1;
                        snap.arb_pnl_usd += net;
                        snap.winning_trades += 1;
                        enforcer_state.current_equity += net;
                        snap.refresh_derived(capital_usd);
                        break;
                    }
                }
            }

            // Whale / intelligence ingestion
            let whale_events: Vec<(u64, WhaleEvent)> = if plan.ingestion.inject_synthetic_whales
                && step % 40 == 0
            {
                let pool = default_pools()[0].pool;
                vec![(
                    t0 + step * TICK_MS * 1_000,
                    WhaleEvent {
                        timestamp_micros: t0 + step * TICK_MS * 1_000,
                        pool_address: pool,
                        swap_amount_usd: 25_000.0,
                        direction: Direction::Long,
                        profitability_score: 0.78,
                    },
                )]
            } else {
                external.write().await.drain_whales()
            };

            for (whale_ts, whale) in whale_events {
                let mut signals =
                    signal_engine.process_at(SignalInput::Whale(whale), whale_ts);
                signals.retain(|s| plan.trade.signal_eligible(s));
                signals.sort_by_key(|s| plan.trade.signal_priority(s));
                for signal in signals {
                    {
                        let mut snap = snapshot.write().await;
                        record_signal_consumed(&mut snap, &signal);
                        snap.signals_emitted += 1;
                    }
                    if let Some(ref cb) = callbacks.on_signal {
                        cb(signal);
                    }
                }
            }
        }

        // ── Risk enforcement ──────────────────────────────────────────────
        match evaluate_limits(&enforcer_state, &enforcer_cfg) {
            RiskAction::Ok => {}
            RiskAction::Pause(_) | RiskAction::PauseDays(_) => {
                let mut snap = snapshot.write().await;
                snap.trading_halted = true;
                snap.halt_reason = Some("daily loss limit reached".to_owned());
                snap.risk_status = "warning".to_owned();
                orch.set_risk_state(RiskState::Locked, "daily loss limit breached");
            }
            RiskAction::PermanentHalt => {
                let mut snap = snapshot.write().await;
                snap.trading_halted = true;
                snap.halt_reason = Some("total loss limit — permanent halt".to_owned());
                snap.risk_status = "critical".to_owned();
                orch.set_risk_state(RiskState::Locked, "total loss halt");
            }
        }

        step = step.wrapping_add(1);
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(Duration::from_millis(TICK_MS)) => {}
        }
    }

    {
        let mut snap = snapshot.write().await;
        snap.running = false;
    }
    info!("autonomous runtime stopped");
    Ok(())
}
