//! Per-source token-bucket rate limiting for free-tier API pollers.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Named API source for rate-limit accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ApiSource {
    Helius,
    Jupiter,
    DexScreener,
    Rugcheck,
    Birdeye,
}

impl ApiSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Helius => "helius",
            Self::Jupiter => "jupiter",
            Self::DexScreener => "dexscreener",
            Self::Rugcheck => "rugcheck",
            Self::Birdeye => "birdeye",
        }
    }
}

struct Bucket {
    tokens: f64,
    max_tokens: f64,
    refill_per_sec: f64,
    last_refill: Instant,
    interval: Duration,
    /// Current poll interval — doubled on 429.
    current_interval: Duration,
}

impl Bucket {
    fn new(max_per_minute: f64, default_interval: Duration) -> Self {
        Self {
            tokens: max_per_minute,
            max_tokens: max_per_minute,
            refill_per_sec: max_per_minute / 60.0,
            last_refill: Instant::now(),
            interval: default_interval,
            current_interval: default_interval,
        }
    }

    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed().as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.max_tokens);
        self.last_refill = Instant::now();
    }

    fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn on_rate_limited(&mut self) {
        self.current_interval = self.current_interval.saturating_mul(2);
        tracing::warn!(
            source = "rate_limit",
            interval_ms = self.current_interval.as_millis(),
            "RateLimitWarning: 429 received — doubled poll interval"
        );
    }

    fn poll_interval(&self) -> Duration {
        self.current_interval
    }
}

/// Simple per-source token bucket. Thread-safe via interior mutex.
pub struct RateLimiter {
    buckets: Mutex<Vec<(ApiSource, Bucket)>>,
}

impl RateLimiter {
    pub fn free_tier_defaults() -> Self {
        Self {
            buckets: Mutex::new(vec![
                (ApiSource::Jupiter, Bucket::new(120.0, Duration::from_millis(500))),
                (ApiSource::DexScreener, Bucket::new(300.0, Duration::from_secs(10))),
                (ApiSource::Rugcheck, Bucket::new(60.0, Duration::from_secs(5))),
                (ApiSource::Birdeye, Bucket::new(100.0, Duration::from_secs(30))),
                (ApiSource::Helius, Bucket::new(50.0, Duration::from_secs(1))),
            ]),
        }
    }

    pub fn try_acquire(&self, source: ApiSource) -> bool {
        let mut guard = self.buckets.lock().expect("ratelimit lock");
        guard
            .iter_mut()
            .find(|(s, _)| *s == source)
            .map(|(_, b)| b.try_acquire())
            .unwrap_or(true)
    }

    pub fn poll_interval(&self, source: ApiSource) -> Duration {
        let guard = self.buckets.lock().expect("ratelimit lock");
        guard
            .iter()
            .find(|(s, _)| *s == source)
            .map(|(_, b)| b.poll_interval())
            .unwrap_or(Duration::from_secs(30))
    }

    pub fn on_rate_limited(&self, source: ApiSource) {
        let mut guard = self.buckets.lock().expect("ratelimit lock");
        if let Some((_, b)) = guard.iter_mut().find(|(s, _)| *s == source) {
            b.on_rate_limited();
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::free_tier_defaults()
    }
}
