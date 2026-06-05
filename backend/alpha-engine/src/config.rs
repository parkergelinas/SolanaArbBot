//! Alpha engine configuration.

use crate::types::ScoringWeights;

#[derive(Clone, Debug)]
pub struct AlphaConfig {
    pub feature_ttl_ms: u64,
    pub wallet_idle_decay_ms: u64,
    pub min_liquidity_usd: f64,
    pub volume_spike_threshold: f64,
    pub weights: ScoringWeights,
    pub min_alpha_score: f64,
    pub min_confidence: f64,
    pub min_liquidity: f64,
    pub cooldown_secs: u64,
    pub queue_max: usize,
    pub queue_ttl_ms: u64,
    pub drain_per_tick: usize,
    pub tick_interval_ms: u64,
    pub base_size_usd: f64,
    pub max_position_usd: f64,
    pub arb_ws_url: Option<String>,
    pub mock_mode: bool,
}

impl AlphaConfig {
    pub fn from_env() -> Self {
        let w1 = env_f64("ALPHA_W1", 0.30);
        let w2 = env_f64("ALPHA_W2", 0.20);
        let w3 = env_f64("ALPHA_W3", 0.25);
        let w4 = env_f64("ALPHA_W4", 0.15);
        let w5 = env_f64("ALPHA_W5", 0.10);
        let sum = w1 + w2 + w3 + w4 + w5;
        let weights = if (sum - 1.0).abs() > 0.01 {
            ScoringWeights {
                w1: w1 / sum,
                w2: w2 / sum,
                w3: w3 / sum,
                w4: w4 / sum,
                w5: w5 / sum,
            }
        } else {
            ScoringWeights { w1, w2, w3, w4, w5 }
        };

        Self {
            feature_ttl_ms: env_u64("ALPHA_FEATURE_TTL_MS", 5000),
            wallet_idle_decay_ms: env_u64("ALPHA_WALLET_IDLE_MS", 30_000),
            min_liquidity_usd: env_f64("ALPHA_MIN_LIQUIDITY_USD", 10_000.0),
            volume_spike_threshold: env_f64("ALPHA_VOLUME_SPIKE_THRESHOLD", 2.0),
            weights,
            min_alpha_score: env_f64("ALPHA_MIN_SCORE", 0.75),
            min_confidence: env_f64("ALPHA_MIN_CONFIDENCE", 0.65),
            min_liquidity: env_f64("ALPHA_MIN_LIQUIDITY", 0.30),
            cooldown_secs: env_u64("SIGNAL_COOLDOWN_SECS", 10),
            queue_max: env_u64("SIGNAL_QUEUE_MAX", 100) as usize,
            queue_ttl_ms: env_u64("SIGNAL_QUEUE_TTL_MS", 5000),
            drain_per_tick: env_u64("SIGNAL_DRAIN_PER_TICK", 5) as usize,
            tick_interval_ms: env_u64("ALPHA_TICK_INTERVAL_MS", 40),
            base_size_usd: env_f64("ALPHA_BASE_SIZE_USD", 100.0),
            max_position_usd: env_f64("MAX_POSITION_USD", 500.0),
            arb_ws_url: std::env::var("ARB_WS_URL").ok(),
            mock_mode: env_bool("ALPHA_MOCK_MODE", true),
        }
    }
}

fn env_f64(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .map(|v| v == "true" || v == "1")
        .unwrap_or(default)
}
