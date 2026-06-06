//! In-process strategy signal bus and dispatcher facade.

use std::sync::{Arc, Mutex};

use config::SystemConfig;
use tokio_util::sync::CancellationToken;
use tracing::debug;

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

struct BusInner {
    subscribers: Vec<crossbeam_channel::Sender<Signal>>,
}

/// Fan-out in-memory bus for strategy publishers (distinct from `signal_bus` crate).
#[derive(Clone)]
pub struct SignalBus {
    inner: Arc<Mutex<BusInner>>,
}

impl SignalBus {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BusInner {
                subscribers: Vec::new(),
            })),
        }
    }

    pub fn publish(&self, signal: Signal) {
        let mut inner = self.inner.lock().expect("signal bus lock");
        inner.subscribers.retain(|tx| tx.send(signal.clone()).is_ok());
        if inner.subscribers.is_empty() {
            tracing::warn!(subsystem = "signal_bus", "no subscribers — signal dropped");
        }
    }

    /// Returns a receiver wired to this bus. Each call adds a new subscriber.
    pub fn subscribe(&self) -> crossbeam_channel::Receiver<Signal> {
        let (tx, rx) = crossbeam_channel::unbounded();
        self.inner
            .lock()
            .expect("signal bus lock")
            .subscribers
            .push(tx);
        rx
    }

    pub fn subscriber_count(&self) -> usize {
        self.inner.lock().expect("signal bus lock").subscribers.len()
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
        if self.bus.subscriber_count() == 0 {
            let rx = self.bus.subscribe();
            let drain_shutdown = shutdown.clone();
            tokio::task::spawn_blocking(move || {
                loop {
                    if drain_shutdown.is_cancelled() {
                        return;
                    }
                    match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                        Ok(signal) => {
                            debug!(
                                subsystem = "signal_bus",
                                strategy = ?signal.strategy,
                                signal_id = %signal.signal_id,
                                expected_pnl_usd = signal.expected_pnl_usd,
                                "strategy signal received"
                            );
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
                    }
                }
            });
        }

        spawn_strategy_publishers(
            Arc::clone(&self.config),
            self.bus.clone(),
            paper_mode,
            shutdown,
        )
    }
}
