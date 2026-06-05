//! Priority queue — highest alpha first, TTL expiration, dedup.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use dashmap::DashMap;

use crate::types::{AlphaSignal, unix_ms};

#[derive(Clone)]
struct QueuedSignal {
    signal: AlphaSignal,
    expires_at: u64,
}

impl PartialEq for QueuedSignal {
    fn eq(&self, other: &Self) -> bool {
        self.signal.score == other.signal.score
    }
}

impl Eq for QueuedSignal {}

impl PartialOrd for QueuedSignal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedSignal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.signal
            .score
            .partial_cmp(&other.signal.score)
            .unwrap_or(Ordering::Equal)
    }
}

pub struct PriorityQueue {
    heap: BinaryHeap<QueuedSignal>,
    dedup: DashMap<(String, String), f64>,
    max_size: usize,
    ttl_ms: u64,
}

impl PriorityQueue {
    pub fn new(max_size: usize, ttl_ms: u64) -> Self {
        Self {
            heap: BinaryHeap::new(),
            dedup: DashMap::new(),
            max_size,
            ttl_ms,
        }
    }

    pub fn push(&mut self, signal: AlphaSignal) -> bool {
        let key = (signal.token_out.clone(), signal.wallet.clone());
        if let Some(existing) = self.dedup.get(&key) {
            if *existing >= signal.score {
                return false;
            }
        }
        self.dedup.insert(key, signal.score);

        let expires_at = unix_ms() + self.ttl_ms;
        self.heap.push(QueuedSignal { signal, expires_at });

        while self.heap.len() > self.max_size {
            if let Some(lowest) = self.heap.pop() {
                let key = (lowest.signal.token_out.clone(), lowest.signal.wallet.clone());
                if self.dedup.get(&key).map(|s| *s == lowest.signal.score).unwrap_or(false) {
                    self.dedup.remove(&key);
                }
            }
        }
        true
    }

    pub fn pop_ready(&mut self, now: u64) -> Option<AlphaSignal> {
        self.evict_expired(now);
        while let Some(top) = self.heap.peek() {
            if top.expires_at <= now {
                let _ = self.heap.pop();
                continue;
            }
            break;
        }
        self.heap.pop().map(|q| q.signal)
    }

    pub fn drain_top_n(&mut self, n: usize, now: u64) -> Vec<AlphaSignal> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(sig) = self.pop_ready(now) {
                out.push(sig);
            } else {
                break;
            }
        }
        out
    }

    pub fn evict_expired(&mut self, now: u64) {
        let mut keep = BinaryHeap::new();
        while let Some(q) = self.heap.pop() {
            if q.expires_at > now {
                keep.push(q);
            }
        }
        self.heap = keep;
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AlphaSignal;

    fn alpha(score: f64, token: &str) -> AlphaSignal {
        AlphaSignal {
            signal_id: format!("s_{score}"),
            token_in: "USDC".into(),
            token_out: token.into(),
            wallet: "w1".into(),
            confidence: score,
            expected_edge: 10.0,
            size_usd: 100.0,
            strategy: "momentum_follow".into(),
            score,
            direction: "long".into(),
            timestamp: unix_ms(),
        }
    }

    #[test]
    fn orders_by_alpha_score() {
        let mut q = PriorityQueue::new(100, 5000);
        q.push(alpha(0.7, "SOL"));
        q.push(alpha(0.9, "SOL"));
        let now = unix_ms();
        let first = q.pop_ready(now).expect("pop");
        assert!((first.score - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn ttl_expiration() {
        let mut q = PriorityQueue::new(100, 10);
        let mut sig = alpha(0.8, "SOL");
        sig.timestamp = unix_ms() - 100;
        q.push(sig);
        let expired = unix_ms() + 20;
        assert!(q.pop_ready(expired).is_none());
    }

    #[test]
    fn dedup_replaces_higher_score() {
        let mut q = PriorityQueue::new(100, 5000);
        assert!(q.push(alpha(0.7, "SOL")));
        assert!(q.push(alpha(0.85, "SOL")));
        assert!(!q.push(alpha(0.6, "SOL")));
    }

    #[test]
    fn max_size_evicts() {
        let mut q = PriorityQueue::new(2, 5000);
        q.push(alpha(0.5, "A"));
        q.push(alpha(0.6, "B"));
        q.push(alpha(0.9, "C"));
        assert!(q.len() <= 2);
    }
}
