//! Ring buffer with dedup TTL and optional JSONL persistence.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::persist::{append_jsonl, load_jsonl};
use crate::types::{AlertType, LiveSignal};

pub const DEFAULT_CAPACITY: usize = 1_000;
pub const DEFAULT_DEDUP_TTL_MS: u64 = 30_000;

#[derive(Clone, Debug)]
pub struct BufferConfig {
    pub capacity: usize,
    pub dedup_ttl_ms: u64,
    pub persist_path: Option<PathBuf>,
}

impl Default for BufferConfig {
    fn default() -> Self {
        let persist = if std::env::var("SIGNAL_BUFFER_PERSIST")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true)
        {
            Some(
                std::env::var("SIGNAL_BUFFER_PATH")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| crate::persist::default_persist_path()),
            )
        } else {
            None
        };
        Self {
            capacity: std::env::var("SIGNAL_BUFFER_CAPACITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_CAPACITY),
            dedup_ttl_ms: std::env::var("SIGNAL_DEDUP_TTL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_DEDUP_TTL_MS),
            persist_path: persist,
        }
    }
}

#[derive(Debug)]
pub struct SignalBuffer {
    store: VecDeque<LiveSignal>,
    dedup_seen: HashMap<String, Instant>,
    config: BufferConfig,
    loaded: bool,
}

impl SignalBuffer {
    pub fn new(config: BufferConfig) -> Self {
        Self {
            store: VecDeque::with_capacity(config.capacity.min(64)),
            dedup_seen: HashMap::new(),
            config,
            loaded: false,
        }
    }

    fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let Some(ref path) = self.config.persist_path else {
            return;
        };
        // Blocking load on first insert — bridge threads call blocking_lock.
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines().rev().take(self.config.capacity) {
                if let Ok(sig) = serde_json::from_str::<LiveSignal>(line) {
                    self.store.push_front(sig);
                }
            }
            while self.store.len() > self.config.capacity {
                self.store.pop_back();
            }
        }
    }

    /// Insert with dedup. Returns `Some(signal)` when accepted.
    pub fn insert(&mut self, signal: LiveSignal) -> Option<LiveSignal> {
        self.ensure_loaded();
        let now = Instant::now();
        let key = signal.dedup_key.clone();
        let ttl = Duration::from_millis(self.config.dedup_ttl_ms);

        self.dedup_seen
            .retain(|_, ts| now.duration_since(*ts) < ttl);
        if let Some(prev) = self.dedup_seen.get(&key) {
            if now.duration_since(*prev) < ttl {
                return None;
            }
        }
        self.dedup_seen.insert(key, now);

        if self.store.len() >= self.config.capacity {
            self.store.pop_front();
        }
        self.store.push_back(signal.clone());

        if let Some(ref path) = self.config.persist_path {
            let path = path.clone();
            let sig = signal.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    rt.block_on(append_jsonl(&path, &sig));
                }
            });
        }

        Some(signal)
    }

    pub fn replay(&self) -> Vec<LiveSignal> {
        self.store.iter().cloned().collect()
    }

    pub fn recent(&self, limit: usize) -> Vec<LiveSignal> {
        self.store
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn recent_filtered(&self, limit: usize, filter: Option<AlertType>) -> Vec<LiveSignal> {
        self.store
            .iter()
            .rev()
            .filter(|s| filter.map_or(true, |t| s.alert_type == Some(t)))
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn clear(&mut self) {
        self.store.clear();
        self.dedup_seen.clear();
    }

    /// Async hydration (control-api startup).
    pub async fn load_persisted_async(&mut self) {
        let Some(ref path) = self.config.persist_path else {
            return;
        };
        let signals = load_jsonl(path, self.config.capacity).await;
        for sig in signals {
            if self.store.len() >= self.config.capacity {
                self.store.pop_front();
            }
            self.store.push_back(sig);
        }
        self.loaded = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SignalKind, SignalSourceMeta, StrategyTag, SCHEMA_VERSION};

    fn sample(key: &str, id: &str) -> LiveSignal {
        LiveSignal {
            v: SCHEMA_VERSION,
            signal_id: id.into(),
            kind: SignalKind::WhaleAlert,
            source: SignalSourceMeta {
                layer: "intelligence".into(),
                dex: "raydium".into(),
                slot: 0,
                wallet_label: None,
            },
            pair: "SOL/BONK".into(),
            token_in: crate::adapter::SOL_MINT.into(),
            token_out: "token".into(),
            timestamp_ms: 1,
            tx_id: id.into(),
            price: 1.0,
            size: 10.0,
            confidence: 0.8,
            wallet: "w".into(),
            strength: Some(0.8),
            size_usd: Some(1000.0),
            alert_type: Some(AlertType::WhaleFlow),
            strategy_tag: Some(StrategyTag::WatchOnly),
            explanation: Some("test".into()),
            direction: Some("Long".into()),
            dedup_key: key.into(),
        }
    }

    #[test]
    fn dedup_collapses_within_ttl() {
        let mut buf = SignalBuffer::new(BufferConfig {
            capacity: 100,
            dedup_ttl_ms: 30_000,
            persist_path: None,
        });
        assert!(buf.insert(sample("k1", "a")).is_some());
        assert!(buf.insert(sample("k1", "b")).is_none());
        assert_eq!(buf.len(), 1);
    }
}
