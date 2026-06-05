//! Execution router — validate signals and dispatch to strategies.

use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use tracing::{info, warn};

use crate::audit::AuditLog;
use crate::config::EngineConfig;
use crate::jupiter::JupiterExecutor;
use crate::orders::{LifecycleEvent, OrderStatus, OrderStore};
use crate::signals::TradeSignal;
use crate::strategy::resolve_strategy;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    LowConfidence { got: f64, min: f64 },
    LowEdge { got: f64, min: f64 },
    SizeTooLarge { got: f64, max: f64 },
    UnknownStrategy,
}

pub struct ExecutionRouter {
    config: EngineConfig,
    orders: Arc<OrderStore>,
    audit: Arc<AuditLog>,
    jupiter: Arc<JupiterExecutor>,
    lifecycle_tx: Option<Sender<LifecycleEvent>>,
}

impl ExecutionRouter {
    pub fn new(
        config: EngineConfig,
        orders: Arc<OrderStore>,
        audit: Arc<AuditLog>,
        lifecycle_tx: Option<Sender<LifecycleEvent>>,
    ) -> Self {
        let jupiter = Arc::new(JupiterExecutor::new(config.clone()));
        Self {
            config,
            orders,
            audit,
            jupiter,
            lifecycle_tx,
        }
    }

    pub fn validate(&self, signal: &TradeSignal) -> Result<(), ValidationError> {
        if signal.confidence < self.config.min_confidence {
            return Err(ValidationError::LowConfidence {
                got: signal.confidence,
                min: self.config.min_confidence,
            });
        }
        if signal.expected_edge < self.config.min_edge_bps {
            return Err(ValidationError::LowEdge {
                got: signal.expected_edge,
                min: self.config.min_edge_bps,
            });
        }
        if signal.size_usd > self.config.max_size_usd {
            return Err(ValidationError::SizeTooLarge {
                got: signal.size_usd,
                max: self.config.max_size_usd,
            });
        }
        Ok(())
    }

    pub async fn process(&self, signal: TradeSignal) {
        if let Err(e) = self.validate(&signal) {
            warn!(?e, strategy = %signal.strategy, "signal rejected");
            return;
        }

        let strategy = resolve_strategy(&signal.strategy);
        if !strategy.evaluate(&signal) {
            warn!(strategy = %signal.strategy, "strategy evaluate failed");
            return;
        }

        let Some(order) = strategy.build_order(&signal) else {
            warn!(strategy = %signal.strategy, "build_order returned None");
            return;
        };

        let created = self.orders.create(signal);
        self.audit
            .log_lifecycle(&LifecycleEvent::Created(created.clone()));
        if let Some(tx) = &self.lifecycle_tx {
            let _ = tx.send(LifecycleEvent::Created(created.clone()));
        }

        let user_pubkey = created.signal.wallet.clone();
        match self.jupiter.execute(&order, &user_pubkey).await {
            Ok(result) => {
                if let Some(submitted) = self.orders.transition(
                    &created.order_id,
                    OrderStatus::Submitted,
                    None,
                    None,
                ) {
                    self.audit.log_transition("pending", "submitted", &submitted);
                }

                let sig = result.tx_signature.clone();
                if let Some(confirmed) = self.orders.transition(
                    &created.order_id,
                    OrderStatus::Confirmed,
                    sig,
                    None,
                ) {
                    self.audit.log_transition("submitted", "confirmed", &confirmed);
                    if let Some(tx) = &self.lifecycle_tx {
                        let _ = tx.send(LifecycleEvent::Updated(confirmed));
                    }
                }

                info!(
                    order_id = %created.order_id,
                    paper = result.paper,
                    strategy = %order.strategy,
                    "execution complete"
                );
            }
            Err(e) => {
                if let Some(failed) = self.orders.transition(
                    &created.order_id,
                    OrderStatus::Failed,
                    None,
                    Some(e.to_string()),
                ) {
                    self.audit.log_transition("pending", "failed", &failed);
                    if let Some(tx) = &self.lifecycle_tx {
                        let _ = tx.send(LifecycleEvent::Updated(failed));
                    }
                }
                warn!(order_id = %created.order_id, error = %e, "execution failed");
            }
        }
    }

    pub fn spawn_worker(self: Arc<Self>, signal_rx: Receiver<TradeSignal>) {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");

            while let Ok(signal) = signal_rx.recv() {
                let router = self.clone();
                rt.block_on(router.process(signal));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditLog;

    fn router() -> ExecutionRouter {
        let config = EngineConfig {
            paper_mode: true,
            min_confidence: 0.6,
            min_edge_bps: 5.0,
            max_size_usd: 100.0,
            jupiter_quote_url: "https://quote-api.jup.ag/v6/quote".into(),
            jupiter_swap_url: "https://quote-api.jup.ag/v6/swap".into(),
            audit_path: None,
            wallet_keypair_path: None,
        };
        ExecutionRouter::new(
            config,
            Arc::new(OrderStore::new()),
            Arc::new(AuditLog::new(None)),
            None,
        )
    }

    #[test]
    fn rejects_low_confidence() {
        let r = router();
        let sig = TradeSignal::new("w", "A", "B", 0.3, 20.0, 10.0, "scalp");
        assert_eq!(
            r.validate(&sig),
            Err(ValidationError::LowConfidence {
                got: 0.3,
                min: 0.6
            })
        );
    }

    #[test]
    fn accepts_valid_signal() {
        let r = router();
        let sig = TradeSignal::new("w", "A", "B", 0.8, 20.0, 10.0, "scalp");
        assert!(r.validate(&sig).is_ok());
    }
}
