//! Runtime configuration for arbitrage detection.

#[derive(Clone, Debug)]
pub struct ArbConfig {
    pub min_spread_pct: f64,
    pub min_liquidity_usd: f64,
    pub max_stale_ms: u64,
    pub min_profit_usd: f64,
    pub min_confidence: f64,
    pub round_trip_fee_pct: f64,
    pub batch_interval_ms: u64,
    pub mock_interval_ms: u64,
    pub ws_port: u16,
    pub ws_enabled: bool,
    pub max_execution_size_usd: f64,
}

impl ArbConfig {
    pub fn from_env() -> Self {
        Self {
            min_spread_pct: env_f64("ARB_MIN_SPREAD_PCT", 0.003),
            min_liquidity_usd: env_f64("ARB_MIN_LIQUIDITY_USD", 10_000.0),
            max_stale_ms: env_u64("ARB_MAX_STALE_MS", 3000),
            min_profit_usd: env_f64("ARB_MIN_PROFIT_USD", 1.0),
            min_confidence: env_f64("ARB_MIN_CONFIDENCE", 0.7),
            round_trip_fee_pct: env_f64("ARB_ROUND_TRIP_FEE_PCT", 0.003),
            batch_interval_ms: env_u64("ARB_BATCH_INTERVAL_MS", 40),
            mock_interval_ms: env_u64("ARB_MOCK_INTERVAL_MS", 50),
            ws_port: env_u64("ARB_WS_PORT", 8091) as u16,
            ws_enabled: env_bool("ARB_WS_ENABLED", true),
            max_execution_size_usd: env_f64("ARB_MAX_EXECUTION_SIZE_USD", 500.0),
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
