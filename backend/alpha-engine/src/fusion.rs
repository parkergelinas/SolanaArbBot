//! Feature fusion engine — incremental DashMap updates per token.

use std::sync::Arc;

use dashmap::DashMap;

use crate::config::AlphaConfig;
use crate::types::{
    ArbSignal, FusedFeatureVector, LiquiditySnapshot, MarketSignal, MicrostructureEvent,
    WalletScore, unix_ms,
};

const EMA_ALPHA: f64 = 0.3;

#[derive(Clone, Default)]
struct TokenState {
    wallet_score: f64,
    wallet_conf: f64,
    wallet_ts: u64,
    wallet_id: String,
    momentum: f64,
    volume_spike: f64,
    price_change: f64,
    imbalance: f64,
    micro_momentum: f64,
    liquidity_usd: f64,
    spread_bps: f64,
    arb_spread: f64,
    arb_conf: f64,
    updated: u64,
}

pub struct FusionEngine {
    config: Arc<AlphaConfig>,
    state: DashMap<String, TokenState>,
    fused: DashMap<String, FusedFeatureVector>,
}

impl FusionEngine {
    pub fn new(config: Arc<AlphaConfig>) -> Self {
        Self {
            config,
            state: DashMap::new(),
            fused: DashMap::new(),
        }
    }

    pub fn on_wallet(&self, ws: &WalletScore) {
        self.update_token(&ws.token, |s| {
            s.wallet_score = ws.score;
            s.wallet_conf = ws.confidence;
            s.wallet_ts = ws.timestamp;
            s.wallet_id = ws.wallet.clone();
            s.updated = unix_ms();
        });
        self.recompute(&ws.token);
    }

    pub fn on_market(&self, ms: &MarketSignal) {
        self.update_token(&ms.token, |s| {
            s.momentum = EMA_ALPHA * ms.momentum + (1.0 - EMA_ALPHA) * s.momentum;
            s.volume_spike = ms.volume_spike;
            s.price_change = ms.price_change;
            s.updated = unix_ms();
        });
        self.recompute(&ms.token);
    }

    pub fn on_microstructure(&self, ev: &MicrostructureEvent) {
        self.update_token(&ev.token, |s| {
            s.imbalance = ev.imbalance;
            s.micro_momentum = ev.momentum;
            s.updated = unix_ms();
        });
        self.recompute(&ev.token);
    }

    pub fn on_arb(&self, arb: &ArbSignal) {
        let token = crate::types::token_from_pair(&arb.token_pair);
        self.update_token(&token, |s| {
            s.arb_spread = arb.spread_pct;
            s.arb_conf = arb.confidence;
            s.updated = unix_ms();
        });
        self.recompute(&token);
    }

    pub fn on_liquidity(&self, liq: &LiquiditySnapshot) {
        self.update_token(&liq.token, |s| {
            s.liquidity_usd = liq.liquidity_usd;
            s.spread_bps = liq.spread_bps;
            s.updated = unix_ms();
        });
        self.recompute(&liq.token);
    }

    fn update_token(&self, token: &str, f: impl FnOnce(&mut TokenState)) {
        let mut entry = self.state.entry(token.to_string()).or_default();
        f(&mut entry);
    }

    fn recompute(&self, token: &str) {
        let Some(s) = self.state.get(token) else {
            return;
        };
        let now = unix_ms();
        let fused = fuse_token(&self.config, token, &s, now);
        self.fused.insert(token.to_string(), fused);
    }

    pub fn get(&self, token: &str) -> Option<FusedFeatureVector> {
        self.evict_stale();
        self.fused.get(token).map(|e| e.clone())
    }

    pub fn evict_stale(&self) {
        let now = unix_ms();
        let ttl = self.config.feature_ttl_ms;
        self.fused.retain(|_, v| now.saturating_sub(v.timestamp) <= ttl);
        self.state.retain(|_, s| now.saturating_sub(s.updated) <= ttl);
    }
}

pub fn fuse_token(
    config: &AlphaConfig,
    token: &str,
    s: &TokenState,
    now: u64,
) -> FusedFeatureVector {
    let wallet_idle = now.saturating_sub(s.wallet_ts);
    let wallet_decay = if wallet_idle > config.wallet_idle_decay_ms {
        0.5
    } else {
        1.0
    };
    let wallet_smart_money_score =
        (s.wallet_score * s.wallet_conf * wallet_decay).clamp(0.0, 1.0);

    let vol_spike_capped = s.volume_spike.min(3.0);
    let volume_spike_norm = (vol_spike_capped / 3.0).clamp(0.0, 1.0);
    let momentum_strength = ((s.momentum.abs() + s.micro_momentum.abs()) / 2.0
        + volume_spike_norm * 0.2
        + s.price_change.abs() * 0.1)
        .clamp(0.0, 1.0);

    let liq_ratio = (s.liquidity_usd / config.min_liquidity_usd).min(1.0);
    let spread_penalty = (s.spread_bps / 100.0).min(0.5);
    let liquidity_conditions = (liq_ratio * (1.0 - spread_penalty)).clamp(0.0, 1.0);

    let arb_raw = s.arb_spread * s.arb_conf;
    let arbitrage_opportunity_strength = (arb_raw / 0.01).min(1.0).max(0.0);

    FusedFeatureVector {
        token: token.to_string(),
        wallet_smart_money_score,
        momentum_strength,
        liquidity_conditions,
        arbitrage_opportunity_strength,
        volume_spike_norm,
        price_change: s.price_change,
        imbalance: s.imbalance,
        timestamp: now,
        ttl_ms: config.feature_ttl_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn cfg() -> Arc<AlphaConfig> {
        Arc::new(AlphaConfig::from_env())
    }

    #[test]
    fn wallet_fusion_updates_score() {
        let engine = FusionEngine::new(cfg());
        engine.on_wallet(&WalletScore {
            wallet: "whale_1".into(),
            score: 0.9,
            confidence: 0.8,
            token: "SOL".into(),
            timestamp: unix_ms(),
        });
        let f = engine.get("SOL").expect("fused");
        assert!(f.wallet_smart_money_score > 0.5);
    }

    #[test]
    fn arb_fusion_sets_arb_strength() {
        let engine = FusionEngine::new(cfg());
        engine.on_arb(&ArbSignal {
            token_pair: "SOL/USDC".into(),
            spread_pct: 0.008,
            confidence: 0.85,
        });
        let f = engine.get("SOL").expect("fused");
        assert!(f.arbitrage_opportunity_strength > 0.0);
    }

    #[test]
    fn stale_eviction_removes_old() {
        let engine = FusionEngine::new(cfg());
        engine.on_market(&MarketSignal {
            token: "SOL".into(),
            momentum: 0.5,
            volume_spike: 2.0,
            price_change: 0.02,
            timestamp: unix_ms() - 10_000,
        });
        engine.evict_stale();
        assert!(engine.get("SOL").is_none());
    }
}
