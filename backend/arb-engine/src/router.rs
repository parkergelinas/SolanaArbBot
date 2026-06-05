//! Routes qualified ArbSignals to output channels (crossbeam + optional WS broadcast).

use std::sync::Arc;

use crossbeam_channel::Sender;
use tracing::{info, instrument};

use crate::config::ArbConfig;
use crate::pool_state::PoolStateEngine;
use crate::signal::{execution_size, passes_routing_filters, TradeSignalOut};
use crate::spread::{compute_spreads, opportunity_to_signal};
use crate::types::ArbSignal;

pub struct ArbRouter {
    config: Arc<ArbConfig>,
    pool_engine: Arc<PoolStateEngine>,
    signal_tx: Sender<TradeSignalOut>,
    arb_tx: Option<Sender<ArbSignal>>,
}

impl ArbRouter {
    pub fn new(
        config: Arc<ArbConfig>,
        pool_engine: Arc<PoolStateEngine>,
        signal_tx: Sender<TradeSignalOut>,
        arb_tx: Option<Sender<ArbSignal>>,
    ) -> Self {
        Self {
            config,
            pool_engine,
            signal_tx,
            arb_tx,
        }
    }

    #[instrument(skip(self), fields(pairs = self.pool_engine.pairs().len()))]
    pub fn run_detection_cycle(&self) -> usize {
        let opps = compute_spreads(&self.pool_engine, &self.config);
        let mut emitted = 0usize;

        for opp in opps {
            let arb = opportunity_to_signal(&opp, &self.config);
            if !passes_routing_filters(&arb, &opp, &self.config) {
                continue;
            }

            let size = execution_size(&opp, &self.config);
            if size < self.config.min_liquidity_usd.min(100.0) {
                continue;
            }

            if let Some(tx) = &self.arb_tx {
                let _ = tx.send(arb.clone());
            }

            let trade = TradeSignalOut::from_arb(&arb, size);
            if self.signal_tx.send(trade).is_ok() {
                emitted += 1;
                info!(
                    pair = %arb.token_pair,
                    spread_pct = arb.spread_pct,
                    confidence = arb.confidence,
                    "arb signal routed"
                );
            }
        }

        emitted
    }
}
