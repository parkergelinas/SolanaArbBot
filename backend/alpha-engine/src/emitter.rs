//! Trade signal emission to execution-engine channel.

use tokio::sync::mpsc;
use tracing::{info, trace};

use crate::types::TradeSignal;

pub struct SignalEmitter {
    tx: mpsc::UnboundedSender<TradeSignal>,
}

impl SignalEmitter {
    pub fn new(tx: mpsc::UnboundedSender<TradeSignal>) -> Self {
        Self { tx }
    }

    pub fn emit(&self, signal: TradeSignal) -> bool {
        trace!(
            token = %signal.token,
            strategy = %signal.strategy,
            confidence = signal.confidence,
            "emitting trade signal"
        );
        match self.tx.send(signal.clone()) {
            Ok(()) => {
                info!(
                    token = %signal.token,
                    direction = %signal.direction,
                    strategy = %signal.strategy,
                    size_usd = signal.size_usd,
                    "trade signal emitted"
                );
                true
            }
            Err(_) => false,
        }
    }
}
