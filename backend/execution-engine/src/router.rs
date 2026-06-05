//! Async execution router — strategy → risk → Jupiter → confirm.

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::audit::AuditLog;
use crate::config::EngineConfig;
use crate::confirmation::ConfirmationService;
use crate::jupiter::JupiterExecutor;
use crate::latency::LatencyTracker;
use crate::orders::{LifecycleEvent, OrderStatus, OrderStore};
use crate::risk::{NoOpVolatilityFilter, RiskGate};
use crate::signals::TradeSignal;
use crate::strategy::{ExecutionTiming, StrategyRegistry};

pub struct ExecutionRouter {
    config: EngineConfig,
    orders: Arc<OrderStore>,
    audit: Arc<AuditLog>,
    jupiter: Arc<JupiterExecutor>,
    risk: Arc<RiskGate>,
    strategies: Arc<StrategyRegistry>,
    confirm: Arc<ConfirmationService>,
    lifecycle_tx: Option<mpsc::UnboundedSender<LifecycleEvent>>,
}

impl ExecutionRouter {
    pub fn new(
        config: EngineConfig,
        orders: Arc<OrderStore>,
        audit: Arc<AuditLog>,
        jupiter: Arc<JupiterExecutor>,
        lifecycle_tx: Option<mpsc::UnboundedSender<LifecycleEvent>>,
    ) -> Self {
        let risk = Arc::new(RiskGate::new(
            &config,
            Arc::new(NoOpVolatilityFilter),
        ));
        Self {
            confirm: Arc::new(ConfirmationService::new(config.paper_mode)),
            config,
            orders,
            audit,
            jupiter,
            risk,
            strategies: Arc::new(StrategyRegistry::new()),
            lifecycle_tx,
        }
    }

    pub async fn process(&self, signal: TradeSignal) {
        let mut lat = LatencyTracker::start();

        let strategy = match self.strategies.get(&signal.strategy) {
            Some(s) => s,
            None => {
                warn!(strategy = %signal.strategy, "unknown strategy");
                return;
            }
        };

        let t0 = Instant::now();
        if let Err(e) = strategy.validate(&signal) {
            warn!(?e, strategy = %signal.strategy, "strategy rejected");
            return;
        }
        let sized = strategy.size_position(&signal, &self.config);
        match strategy.timing(&signal) {
            ExecutionTiming::Skip => {
                warn!(strategy = %signal.strategy, "timing skip");
                return;
            }
            ExecutionTiming::DelayedMs(ms) => {
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            }
            ExecutionTiming::Immediate => {}
        }
        lat.record_strategy(t0);

        let t1 = Instant::now();
        if let Err(e) = self.risk.evaluate(&signal, strategy.as_ref(), &sized) {
            warn!(?e, "risk rejected");
            return;
        }
        lat.record_risk(t1);

        let created = self.orders.create(signal);
        self.audit
            .log_lifecycle(&LifecycleEvent::Created(created.clone()));
        if let Some(tx) = &self.lifecycle_tx {
            let _ = tx.send(LifecycleEvent::Created(created.clone()));
        }

        let t2 = Instant::now();
        let quote = match self.jupiter.get_quote(&sized).await {
            Ok(q) => {
                lat.record_quote(t2);
                q
            }
            Err(e) => {
                if let Some(failed) = self.orders.transition(
                    &created.order_id,
                    OrderStatus::Failed,
                    None,
                    Some(e.to_string()),
                ) {
                    self.audit.log_transition("pending", "failed", &failed);
                }
                warn!(order_id = %created.order_id, error = %e, "jupiter quote failed");
                return;
            }
        };

        let t3 = Instant::now();
        let exec_result = match self.jupiter.execute_with_quote(&sized, quote).await {
            Ok(r) => {
                lat.record_swap_build(t3);
                r
            }
            Err(e) => {
                if let Some(failed) = self.orders.transition(
                    &created.order_id,
                    OrderStatus::Failed,
                    None,
                    Some(e.to_string()),
                ) {
                    self.audit.log_transition("pending", "failed", &failed);
                }
                warn!(order_id = %created.order_id, error = %e, "jupiter failed");
                return;
            }
        };

        if let Some(submitted) = self.orders.transition(
            &created.order_id,
            OrderStatus::Submitted,
            None,
            None,
        ) {
            self.audit.log_transition("pending", "submitted", &submitted);
        }

        let sig = exec_result.tx_signature.clone().unwrap_or_default();
        if self.confirm.confirm(&sig).await {
            if let Some(confirmed) = self.orders.transition(
                &created.order_id,
                OrderStatus::Confirmed,
                exec_result.tx_signature.clone(),
                None,
            ) {
                self.audit.log_transition("submitted", "confirmed", &confirmed);
                if let Some(tx) = &self.lifecycle_tx {
                    let _ = tx.send(LifecycleEvent::Updated(confirmed));
                }
            }
            self.risk.record_fill(&sized.output_mint);
        }

        let budget = lat.finish();
        info!(
            order_id = %created.order_id,
            paper = exec_result.paper,
            strategy = %sized.strategy,
            total_ms = budget.total_ms,
            "execution complete"
        );
    }

    pub fn spawn(self: Arc<Self>, mut signal_rx: mpsc::UnboundedReceiver<TradeSignal>) {
        tokio::spawn(async move {
            while let Some(signal) = signal_rx.recv().await {
                let router = self.clone();
                tokio::spawn(async move {
                    router.process(signal).await;
                });
            }
        });
    }
}
