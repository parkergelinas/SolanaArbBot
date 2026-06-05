//! Structured audit logging + optional JSONL file sink.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

use tracing::info;

use crate::orders::{LifecycleEvent, OrderRecord};

pub struct AuditLog {
    path: Option<String>,
    file: Mutex<Option<std::fs::File>>,
}

impl AuditLog {
    pub fn new(path: Option<String>) -> Self {
        let file = path.as_ref().and_then(|p| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .ok()
        });
        Self {
            path,
            file: Mutex::new(file),
        }
    }

    pub fn log_lifecycle(&self, event: &LifecycleEvent) {
        let record = match event {
            LifecycleEvent::Created(r) | LifecycleEvent::Updated(r) => r,
        };

        info!(
            order_id = %record.order_id,
            status = ?record.status,
            strategy = %record.signal.strategy,
            size_usd = record.signal.size_usd,
            paper = true,
            "order lifecycle"
        );

        if let Some(path) = &self.path {
            if let Ok(json) = serde_json::to_string(record) {
                if let Ok(mut guard) = self.file.lock() {
                    if guard.is_none() {
                        *guard = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path)
                            .ok();
                    }
                    if let Some(f) = guard.as_mut() {
                        let _ = writeln!(f, "{json}");
                    }
                }
            }
        }
    }

    pub fn log_transition(&self, from: &str, to: &str, order: &OrderRecord) {
        info!(
            order_id = %order.order_id,
            from,
            to,
            "order state transition"
        );
        self.log_lifecycle(&LifecycleEvent::Updated(order.clone()));
    }
}
