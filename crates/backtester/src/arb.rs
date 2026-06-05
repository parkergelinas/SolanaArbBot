//! DEX-to-DEX arbitrage backtest runner.

use std::collections::HashMap;
use std::sync::Arc;

use common::Pubkey;
use config::SystemConfig;
use decoder::{DexDecoder, DexType, PoolState};
use execution::ExecutionSimulator;
use graph::{Edge, MarketGraph};
use pricing::PricingEngine;
use risk::RiskEngine;
use routing::{Route, Router};

use crate::dataset::{ReplayDataset, ReplayEvent};
use crate::metrics::StrategyMetrics;
use common::MarketEvent;

const NETWORK_COST_USD: f64 = 0.05;
/// Haircut applied to theoretical spread to approximate AMM slippage in backtests.
const SLIPPAGE_HAIRCUT: f64 = 0.20;

fn theoretical_profit(route: &Route, trade_size: f64) -> f64 {
    let product = route.path.iter().fold(1.0_f64, |acc, edge| {
        acc * edge.price * (1.0 - edge.fee_bps as f64 / 10_000.0)
    });
    trade_size * (product - 1.0)
}

fn add_pool_edges(graph: &mut MarketGraph, pool: &PoolState) -> common::Result<()> {
    let liquidity = pool.liquidity as f64;
    let (forward_price, reverse_price) = match pool.reserves {
        Some((reserve_a, reserve_b)) if reserve_a > 0 && reserve_b > 0 => (
            reserve_b as f64 / reserve_a as f64,
            reserve_a as f64 / reserve_b as f64,
        ),
        _ => (1.0, 1.0),
    };

    graph.add_edge(Edge::new(
        pool.token_a.clone(),
        pool.token_b.clone(),
        forward_price,
        liquidity,
        25,
    ))?;
    graph.add_edge(Edge::new(
        pool.token_b.clone(),
        pool.token_a.clone(),
        reverse_price,
        liquidity,
        25,
    ))
}

fn rebuild_graph(pools: &HashMap<Pubkey, PoolState>) -> MarketGraph {
    let mut graph = MarketGraph::new();
    for pool in pools.values() {
        let _ = add_pool_edges(&mut graph, pool);
    }
    graph
}

/// Runs DEX-to-DEX arb backtest over `dataset`.
pub fn run_arb_backtest(cfg: Arc<SystemConfig>, dataset: &ReplayDataset) -> StrategyMetrics {
    let decoder = DexDecoder::new();
    let pricing = PricingEngine::new(decoder);
    let risk = RiskEngine::new();

    let mut pool_states: HashMap<Pubkey, PoolState> = HashMap::new();

    let trade_size = cfg.execution.simulation_initial_amount_usd;
    let min_profit = cfg.execution.min_profit_threshold_usd;

    let mut winning = 0u64;
    let mut losing = 0u64;
    let rejected = 0u64;
    let mut total = 0u64;
    let mut net_pnl = 0.0_f64;
    let mut gross_pnl = 0.0_f64;
    let mut fees = 0.0_f64;
    let mut equity = vec![0.0_f64];
    let mut last_trade_ts: u64 = 0;
    let cooldown_us = cfg.scalper.trade_cooldown_secs * 1_000_000;

    for (ts, event) in &dataset.events {
        let ReplayEvent::Market(me) = event else {
            continue;
        };
        let MarketEvent::PoolUpdate(update) = me else {
            continue;
        };
        let Some(pool_key) = update.pool else {
            continue;
        };

        let dex = if pool_key.as_bytes()[1] == 0 {
            DexType::Raydium
        } else {
            DexType::OrcaCLMM
        };

        if let Ok(pool) = decoder.decode_event(dex, me) {
            pool_states.insert(pool_key, pool);
        }

        let graph = rebuild_graph(&pool_states);
        let router = Router::new(graph, pricing);
        let execution = ExecutionSimulator::new(router.clone());

        if *ts < last_trade_ts.saturating_add(cooldown_us) {
            continue;
        }

        let routes = router.find_arbitrage_cycles();
        let mut traded_this_tick = false;

        for route in routes {
            if traded_this_tick {
                break;
            }

            let gross_edge = theoretical_profit(&route, trade_size);
            if gross_edge <= 0.0 {
                continue;
            }

            let Ok(result) = execution.simulate_route(&route, trade_size) else {
                continue;
            };

            if !risk.evaluate_route(&route, &result).allowed {
                continue;
            }

            let hop_fees = trade_size * 0.005;
            let gross_after_slippage = gross_edge * (1.0 - SLIPPAGE_HAIRCUT);
            let net = gross_after_slippage - hop_fees - NETWORK_COST_USD;

            if net < min_profit {
                continue;
            }

            total += 1;
            last_trade_ts = *ts;
            traded_this_tick = true;
            gross_pnl += gross_after_slippage;
            fees += hop_fees + NETWORK_COST_USD;
            net_pnl += net;

            if net > 0.0 {
                winning += 1;
            } else {
                losing += 1;
            }
            equity.push(net_pnl);
        }
    }

    let mut metrics = StrategyMetrics {
        strategy: "dex_arb".to_owned(),
        total_trades: total,
        winning_trades: winning,
        losing_trades: losing,
        rejected_trades: rejected,
        gross_pnl_usd: gross_pnl,
        net_pnl_usd: net_pnl,
        total_fees_usd: fees,
        win_rate: 0.0,
        avg_net_per_trade_usd: 0.0,
        trades_per_hour: 0.0,
        trades_per_day: 0.0,
        max_drawdown_usd: 0.0,
        sharpe_approx: 0.0,
    };
    metrics.finalize(dataset.duration_hours(), &equity);
    metrics
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Token;
    use graph::Edge;
    use routing::Route;

    #[test]
    fn theoretical_profit_detects_cross_dex_spread() {
        let usdc = Token::new(common::Pubkey::new([0; 32]), 6, None);
        let sol = Token::new(common::Pubkey::new([1; 32]), 9, None);

        let route = Route {
            path: vec![
                Edge::new(usdc.clone(), sol.clone(), 0.006901, 2_000_000.0, 25),
                Edge::new(sol.clone(), usdc.clone(), 149.25, 2_000_000.0, 25),
            ],
            score: 1.0,
        };

        let profit = theoretical_profit(&route, 200.0);
        assert!(profit > 4.0, "expected positive cross-dex profit, got {profit}");
    }
}
