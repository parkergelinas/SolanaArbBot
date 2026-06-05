//! `SignalBus` — thread-safe facade over [`SignalBuffer`] with broadcast subscribe.

use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use crate::buffer::{BufferConfig, SignalBuffer};
use crate::types::{AlertType, LiveSignal};

const NOTIFY_CAPACITY: usize = 1024;

/// Shared live signal bus — single source of truth for API consumers.
#[derive(Clone)]
pub struct SignalBus {
    inner: Arc<Mutex<SignalBuffer>>,
    notify: broadcast::Sender<LiveSignal>,
}

impl SignalBus {
    pub fn with_defaults() -> Self {
        Self::new(BufferConfig::default())
    }

    pub fn new(config: BufferConfig) -> Self {
        let (notify, _) = broadcast::channel(NOTIFY_CAPACITY);
        Self {
            inner: Arc::new(Mutex::new(SignalBuffer::new(config))),
            notify,
        }
    }

    pub async fn load_persisted(&self) {
        if let Ok(mut buf) = self.inner.lock() {
            buf.load_persisted_async().await;
        }
    }

    /// Blocking publish — safe from data-layer bridge threads.
    pub fn publish_blocking(&self, signal: LiveSignal) -> Option<LiveSignal> {
        let accepted = self.inner.lock().ok()?.insert(signal)?;
        let _ = self.notify.send(accepted.clone());
        Some(accepted)
    }

    /// Async publish wrapper.
    pub async fn publish(&self, signal: LiveSignal) -> Option<LiveSignal> {
        let bus = self.clone();
        tokio::task::spawn_blocking(move || bus.publish_blocking(signal))
            .await
            .ok()?
    }

    pub fn replay_blocking(&self) -> Vec<LiveSignal> {
        self.inner
            .lock()
            .map(|b| b.replay())
            .unwrap_or_default()
    }

    pub async fn replay(&self) -> Vec<LiveSignal> {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || inner.lock().map(|b| b.replay()).unwrap_or_default())
            .await
            .unwrap_or_default()
    }

    pub async fn recent(&self, limit: usize) -> Vec<LiveSignal> {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || inner.lock().map(|b| b.recent(limit)).unwrap_or_default())
            .await
            .unwrap_or_default()
    }

    pub async fn recent_filtered(&self, limit: usize, filter: AlertType) -> Vec<LiveSignal> {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || {
            inner
                .lock()
                .map(|b| b.recent_filtered(limit, Some(filter)))
                .unwrap_or_default()
        })
        .await
        .unwrap_or_default()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LiveSignal> {
        self.notify.subscribe()
    }

    pub async fn len(&self) -> usize {
        let inner = self.inner.clone();
        tokio::task::spawn_blocking(move || inner.lock().map(|b| b.len()).unwrap_or(0))
            .await
            .unwrap_or(0)
    }
}
