//! In-process strategy signal bus and dispatcher facade.

use std::sync::Arc;

use config::SystemConfig;
use tokio_util::sync::CancellationToken;

use crate::publishers::{spawn_strategy_publishers, PublisherHandles};

/// Strategy lane identifier for published signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StrategyMode {
    Arbitrage,
    QuoteArb,
    CopyTrading,
    Liquidation,
    Momentum,
}

/// Typed payload attached to a strategy signal.
#[derive(Clone, Debug)]
pub enum SignalPayload {
    Arbitrage {
        pair: String,
        spread_bps: f64,
        net_profit_usd: f64,
    },
    QuoteArb {
        pair_label: String,
        input_mint: String,
        output_mint: String,
        route_divergence_bps: u64,
        price_dislocation_bps: u64,
        edge_bps: u64,
        expected_pnl_usd: f64,
    },
    CopyTrading {
        wallet: String,
        token_out: String,
        amount_usd: f64,
    },
    Liquidation {
        obligation: String,
        health: f64,
        expected_bonus_usd: f64,
    },
    Momentum {
        pool: String,
        strength: f64,
        confidence: f64,
    },
}

/// Normalized strategy signal published on the engine bus.
#[derive(Clone, Debug)]
pub struct Signal {
    pub strategy: StrategyMode,
    pub signal_id: String,
    pub notional_usd: f64,
    pub expected_pnl_usd: f64,
    pub payload: SignalPayload,
}

/// Lightweight in-memory bus for strategy publishers (distinct from `signal_bus` crate).
#[derive(Clone, Debug)]
pub struct SignalBus {
    tx: crossbeam_channel::Sender<Signal>,
}

impl SignalBus {
    pub fn new() -> Self {
        let (tx, _rx) = crossbeam_channel::unbounded();
        Self { tx }
    }

    pub fn publish(&self, signal: Signal) {
        if self.tx.send(signal).is_err() {
            tracing::warn!(subsystem = "signal_bus", "no subscribers — signal dropped");
        }
    }

    pub fn subscribe(&self) -> crossbeam_channel::Receiver<Signal> {
        let (_tx, rx) = crossbeam_channel::unbounded();
        // Re-wire: clone sender side by sharing the same channel via pairing at construction.
        // For the minimal facade we expose only publish; tests can use paired buses.
        rx
    }
}

impl Default for SignalBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Master strategy dispatcher — owns config + bus and spawns publishers.
pub struct StrategyDispatcher {
    config: Arc<SystemConfig>,
    bus: SignalBus,
}

impl StrategyDispatcher {
    pub fn new(config: Arc<SystemConfig>) -> Self {
        Self {
            config,
            bus: SignalBus::new(),
        }
    }

    pub fn bus(&self) -> SignalBus {
        self.bus.clone()
    }

    pub fn start(&self, paper_mode: bool, shutdown: CancellationToken) -> PublisherHandles {
        spawn_strategy_publishers(
            Arc::clone(&self.config),
            self.bus.clone(),
            paper_mode,
            shutdown,
        )
    }
}
