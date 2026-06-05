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
use signals::{SignalEngine, SignalInput, SignalEvent, WhaleEvent, Direction};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::ingestion::{default_pools, tick_events};
use crate::snapshot::RuntimeSnapshot;

const TICK_MS: u64 = 500;
const NETWORK_COST_USD: f64 = 0.05;
const SLIPPAGE_HAIRCUT: f64 = 0.20;

pub struct AutonomousCallbacks {
    pub on_signal: Option<Arc<dyn Fn(SignalEvent) + Send + Sync>>,
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

/// Spawns the autonomous paper-trading runtime.
pub fn spawn_autonomous_runtime(
    config: Arc<SystemConfig>,
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
    cancel: CancellationToken,
    callbacks: AutonomousCallbacks,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_loop(config, snapshot, cancel, callbacks).await {
            warn!(error = %e, "autonomous runtime exited with error");
        }
    })
}

async fn run_loop(
    config: Arc<SystemConfig>,
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
    cancel: CancellationToken,
    callbacks: AutonomousCallbacks,
) -> Result<(), String> {
    let (orch, state_rx) = Orchestrator::new(Arc::clone(&config));
    orch.enable_trading("autonomous runtime started")
        .map_err(|e| format!("{e}"))?;

    let (signal_engine, _rx) = SignalEngine::new(config.signal_engine.clone());
    let scalp_engine = ScalpEngine::new(Arc::clone(&config));
    let decoder = DexDecoder::new();
    let pricing = PricingEngine::new(decoder);
    let risk = RiskEngine::new();

    let mut pool_states: HashMap<Pubkey, PoolState> = HashMap::new();
    let mut last_arb_ts: u64 = 0;
    let arb_cooldown_us = config.scalper.trade_cooldown_secs * 1_000_000;
    let trade_size = config.execution.simulation_initial_amount_usd;
    let min_profit = config.execution.min_profit_threshold_usd;

    let mut enforcer_state = EnforcerState::new(config.risk.capital_usd);
    let enforcer_cfg = EnforcerConfig {
        capital_usd: config.risk.capital_usd,
        daily_max_loss_pct: config.risk.daily_loss_limit_pct,
        monthly_max_loss_pct: config.risk.monthly_loss_limit_pct,
        total_max_loss_pct: config.risk.total_loss_halt_pct,
        max_drawdown_pct: config.risk.max_drawdown_pct,
        daily_pause: config.risk.daily_pause(),
    };

    {
        let mut snap = snapshot.write().await;
        snap.running = true;
        snap.mode = "paper".to_owned();
        snap.scalp_enabled = true;
        snap.arb_enabled = true;
        snap.peak_equity_usd = config.risk.capital_usd;
        snap.current_equity_usd = config.risk.capital_usd;
        snap.risk_status = "healthy".to_owned();
        snap.risk_state = "normal".to_owned();
    }

    info!("autonomous runtime started — scalping + dex-to-dex (paper)");

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

        if !halted {
            let ts = t0 + step * TICK_MS * 1_000;
            for (evt_ts, market_evt) in tick_events(step, ts) {
                {
                    let mut snap = snapshot.write().await;
                    snap.events_processed += 1;
                }

                // ── Scalping path ─────────────────────────────────────────
                let signals = signal_engine.process_at(
                    SignalInput::Market(market_evt.clone()),
                    evt_ts,
                );
                if !signals.is_empty() {
                    let mut snap = snapshot.write().await;
                    snap.signals_emitted += signals.len() as u64;
                }

                for signal in signals {
                    if let Some(ref cb) = callbacks.on_signal {
                        cb(signal.clone());
                    }

                    let pool_byte = signal.pool_address.as_bytes()[0];
                    let features = signal_engine.feature_store().compute(
                        signal.pool_address,
                        evt_ts,
                        &config.signal_engine,
                    );

                    let gate_req = ExecutionApprovalRequest {
                        signal_id: signal.signal_id,
                        pool_address: format!("{:?}", signal.pool_address),
                        trade_size_usd: config.scalper.min_position_usd,
                        estimated_cost_bps: 30.0,
                        net_edge_bps: signal.strength * 10_000.0,
                        requested_at_micros: evt_ts,
                    };
                    let gate = orch.approve_execution(&gate_req);
                    if !gate.approved {
                        continue;
                    }

                    if let Some(result) =
                        scalp_engine.evaluate(signal, features, pool_tvl(pool_byte), evt_ts)
                    {
                        if !result.rejected {
                            let pnl_usd = result.candidate.trade_size_usd
                                * (result.pnl_bps / 10_000.0);
                            let fee_usd = result.candidate.trade_size_usd
                                * (result.fee_paid_bps * 2.0 / 10_000.0);
                            let net = pnl_usd - fee_usd;

                            let mut snap = snapshot.write().await;
                            snap.scalp_trades += 1;
                            snap.scalp_pnl_usd += net;
                            if net > 0.0 {
                                snap.winning_trades += 1;
                            }
                            enforcer_state.current_equity += net;
                            if net < 0.0 {
                                enforcer_state.cumulative_daily_loss += net;
                                snap.daily_loss_usd += net.abs();
                            }
                            snap.refresh_derived(config.risk.capital_usd);
                        }
                    }
                }

                // ── DEX-to-DEX arb path ───────────────────────────────────
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
                        let gross_edge = theoretical_profit(&route, trade_size);
                        if gross_edge <= 0.0 {
                            continue;
                        }
                        let Ok(sim) = execution.simulate_route(&route, trade_size) else {
                            continue;
                        };
                        if !risk.evaluate_route(&route, &sim).allowed {
                            continue;
                        }

                        let gate_req = ExecutionApprovalRequest {
                            signal_id: step,
                            pool_address: "arb-cycle".to_owned(),
                            trade_size_usd: trade_size,
                            estimated_cost_bps: 50.0,
                            net_edge_bps: (gross_edge / trade_size) * 10_000.0,
                            requested_at_micros: evt_ts,
                        };
                        if !orch.approve_execution(&gate_req).approved {
                            continue;
                        }

                        let net = gross_edge * (1.0 - SLIPPAGE_HAIRCUT)
                            - trade_size * 0.005
                            - NETWORK_COST_USD;
                        if net < min_profit {
                            continue;
                        }

                        last_arb_ts = evt_ts;
                        let mut snap = snapshot.write().await;
                        snap.arb_trades += 1;
                        snap.arb_pnl_usd += net;
                        snap.winning_trades += 1;
                        enforcer_state.current_equity += net;
                        snap.refresh_derived(config.risk.capital_usd);
                        break;
                    }
                }
            }

            // Whale events for smart-money / whale signals
            if step % 40 == 0 {
                let pool = default_pools()[0].pool;
                let whale = WhaleEvent {
                    timestamp_micros: t0 + step * TICK_MS * 1_000,
                    pool_address: pool,
                    swap_amount_usd: 25_000.0,
                    direction: Direction::Long,
                    profitability_score: 0.78,
                };
                let signals =
                    signal_engine.process_at(SignalInput::Whale(whale), t0 + step * TICK_MS * 1_000);
                for signal in signals {
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
