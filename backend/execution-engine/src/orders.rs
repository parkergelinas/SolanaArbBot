//! Order lifecycle store and state machine.

use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::signals::{TradeSignal, unix_ms};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderRecord {
    pub v: u32,
    pub order_id: String,
    pub signal: TradeSignal,
    pub status: OrderStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug)]
pub enum LifecycleEvent {
    Created(OrderRecord),
    Updated(OrderRecord),
}

pub struct OrderStore {
    inner: Arc<DashMap<String, OrderRecord>>,
}

impl OrderStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    pub fn create(&self, signal: TradeSignal) -> OrderRecord {
        let now = unix_ms();
        let record = OrderRecord {
            v: SCHEMA_VERSION,
            order_id: format!("ord_{}", Uuid::new_v4()),
            signal,
            status: OrderStatus::Pending,
            tx_signature: None,
            error: None,
            created_at: now,
            updated_at: now,
        };
        self.inner.insert(record.order_id.clone(), record.clone());
        record
    }

    pub fn transition(
        &self,
        order_id: &str,
        status: OrderStatus,
        tx_signature: Option<String>,
        error: Option<String>,
    ) -> Option<OrderRecord> {
        let mut updated = None;
        self.inner
            .entry(order_id.to_string())
            .and_modify(|r| {
                r.status = status;
                if tx_signature.is_some() {
                    r.tx_signature = tx_signature.clone();
                }
                if error.is_some() {
                    r.error = error.clone();
                }
                r.updated_at = unix_ms();
                updated = Some(r.clone());
            });
        updated
    }

    pub fn get(&self, order_id: &str) -> Option<OrderRecord> {
        self.inner.get(order_id).map(|r| r.clone())
    }

    pub fn all(&self) -> Vec<OrderRecord> {
        self.inner.iter().map(|r| r.value().clone()).collect()
    }
}

impl Default for OrderStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_lifecycle_pending_to_confirmed() {
        let store = OrderStore::new();
        let sig = TradeSignal::new("SOL", "long", 0.8, 25.0, 50.0, "momentum_follow");
        let created = store.create(sig);
        assert_eq!(created.status, OrderStatus::Pending);

        let submitted = store
            .transition(&created.order_id, OrderStatus::Submitted, None, None)
            .unwrap();
        assert_eq!(submitted.status, OrderStatus::Submitted);

        let confirmed = store
            .transition(
                &created.order_id,
                OrderStatus::Confirmed,
                Some("paper_sig".into()),
                None,
            )
            .unwrap();
        assert_eq!(confirmed.status, OrderStatus::Confirmed);
        assert_eq!(confirmed.tx_signature.as_deref(), Some("paper_sig"));
    }
}
