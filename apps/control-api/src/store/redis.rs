//! Upstash / Redis integration stub.
//!
//! control-api currently keeps signals and trade journal in memory. When running
//! multiple replicas or surviving restarts, wire `UPSTASH_REDIS_REST_URL` here.
//!
//! Full implementation is deferred — this module validates the URL and logs intent.

use tracing::info;

#[derive(Clone, Debug)]
pub struct RedisStoreConfig {
    pub url: String,
}

impl RedisStoreConfig {
    pub fn from_env() -> Option<Self> {
        std::env::var("UPSTASH_REDIS_REST_URL")
            .or_else(|_| std::env::var("REDIS_URL"))
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .map(|url| Self { url })
    }

    /// Log that Redis is configured; actual client wiring is a future phase.
    pub fn announce_stub(&self) {
        info!(
            subsystem = "redis_store",
            url_prefix = %self.url.chars().take(24).collect::<String>(),
            "UPSTASH_REDIS_REST_URL set — in-memory state remains active; \
             Redis persistence not yet implemented (see docs/deploy-control-api.md)"
        );
    }
}
