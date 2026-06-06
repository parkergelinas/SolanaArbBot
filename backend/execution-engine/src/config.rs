//! Runtime configuration — paper mode enforced by default.

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub paper_mode: bool,
    pub execution_live: bool,
    pub min_confidence: f64,
    pub min_edge_bps: f64,
    pub max_position_usd: f64,
    pub default_slippage_bps: u32,
    pub token_cooldown_secs: u64,
    pub request_timeout_ms: u64,
    pub jupiter_base_url: String,
    pub audit_path: Option<String>,
    pub wallet_pubkey: String,
    pub quote_cache_ttl_ms: u64,
    pub quote_mint: String,
    pub base_mint: String,
}

impl EngineConfig {
    pub fn from_env() -> Self {
        let execution_live = std::env::var("EXECUTION_LIVE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let paper_mode = std::env::var("PAPER_MODE")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        Self {
            paper_mode: paper_mode && !execution_live,
            execution_live,
            min_confidence: env_f64("MIN_SIGNAL_CONFIDENCE", 0.55),
            min_edge_bps: env_f64("MIN_EXPECTED_EDGE_BPS", 5.0),
            max_position_usd: env_f64("MAX_POSITION_USD", 500.0),
            default_slippage_bps: env_u64("DEFAULT_SLIPPAGE_BPS", 50) as u32,
            token_cooldown_secs: env_u64("TOKEN_COOLDOWN_SECS", 30),
            request_timeout_ms: env_u64("REQUEST_TIMEOUT_MS", 800),
            jupiter_base_url: std::env::var("JUPITER_BASE_URL")
                .unwrap_or_else(|_| "https://api.jup.ag/swap/v1".into()),
            audit_path: std::env::var("EXECUTION_AUDIT_PATH").ok(),
            wallet_pubkey: std::env::var("EXECUTION_WALLET_PUBKEY")
                .unwrap_or_else(|_| "11111111111111111111111111111111".into()),
            quote_cache_ttl_ms: env_u64("QUOTE_CACHE_TTL_MS", 200),
            quote_mint: std::env::var("QUOTE_MINT")
                .unwrap_or_else(|_| "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into()),
            base_mint: std::env::var("BASE_MINT")
                .unwrap_or_else(|_| "So11111111111111111111111111111111111111112".into()),
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
