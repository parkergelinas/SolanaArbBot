//! Alpha scoring model — weighted fusion → AlphaSignal.

use uuid::Uuid;

use crate::config::AlphaConfig;
use crate::types::{AlphaSignal, FusedFeatureVector};

pub fn compute_alpha_score(fused: &FusedFeatureVector, config: &AlphaConfig) -> f64 {
    let w = &config.weights;
    let raw = w.w1 * fused.wallet_smart_money_score
        + w.w2 * fused.momentum_strength
        + w.w3 * fused.arbitrage_opportunity_strength
        + w.w4 * fused.volume_spike_norm
        + w.w5 * fused.liquidity_conditions;

    let clamped = raw.clamp(0.0, 1.0);
    sigmoid_squash(clamped)
}

fn sigmoid_squash(x: f64) -> f64 {
    let k = 6.0;
    1.0 / (1.0 + (-k * (x - 0.5)).exp())
}

pub fn select_strategy(fused: &FusedFeatureVector) -> &'static str {
    if fused.arbitrage_opportunity_strength > 0.6 {
        "arbitrage_capture"
    } else if fused.wallet_smart_money_score > 0.7 {
        "whale_copy_trade"
    } else if fused.momentum_strength > 0.65 && fused.volume_spike_norm > 0.5 {
        "sniper_entry"
    } else if fused.momentum_strength > 0.5 {
        "momentum_follow"
    } else {
        "momentum_follow"
    }
}

pub fn score_to_alpha(
    fused: &FusedFeatureVector,
    config: &AlphaConfig,
    wallet: &str,
) -> Option<AlphaSignal> {
    if fused.liquidity_conditions < config.min_liquidity {
        return None;
    }

    let score = compute_alpha_score(fused, config);
    if score < config.min_alpha_score {
        return None;
    }

    let strategy = select_strategy(fused);
    let direction = if fused.momentum_strength >= 0.0 && fused.imbalance >= 0.0 {
        "long"
    } else {
        "short"
    };

    let expected_edge = fused.arbitrage_opportunity_strength * fused.price_change.abs() * 100.0
        + fused.momentum_strength * 1.0;

    let size_usd = (config.base_size_usd * score * fused.liquidity_conditions)
        .min(config.max_position_usd);

    let token_out = fused.token.clone();
    let token_in = if direction == "long" {
        "USDC".to_string()
    } else {
        fused.token.clone()
    };

    Some(AlphaSignal {
        signal_id: format!("alpha_{}", Uuid::new_v4()),
        token_in,
        token_out,
        wallet: wallet.to_string(),
        confidence: score,
        expected_edge,
        size_usd,
        strategy: strategy.to_string(),
        score,
        direction: direction.to_string(),
        timestamp: fused.timestamp,
    })
}

pub fn normalize_weights(w1: f64, w2: f64, w3: f64, w4: f64, w5: f64) -> [f64; 5] {
    let sum = w1 + w2 + w3 + w4 + w5;
    if sum <= 0.0 {
        return [0.2; 5];
    }
    [w1 / sum, w2 / sum, w3 / sum, w4 / sum, w5 / sum]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AlphaConfig;

    fn fused(high_arb: bool) -> FusedFeatureVector {
        FusedFeatureVector {
            token: "SOL".into(),
            wallet_smart_money_score: 0.8,
            momentum_strength: 0.7,
            liquidity_conditions: 0.6,
            arbitrage_opportunity_strength: if high_arb { 0.8 } else { 0.2 },
            volume_spike_norm: 0.6,
            price_change: 0.03,
            imbalance: 0.1,
            timestamp: crate::types::unix_ms(),
            ttl_ms: 5000,
        }
    }

    #[test]
    fn alpha_score_in_range() {
        let cfg = AlphaConfig::from_env();
        let score = compute_alpha_score(&fused(false), &cfg);
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn weight_normalization_sums_to_one() {
        let w = normalize_weights(0.3, 0.2, 0.25, 0.15, 0.1);
        let sum: f64 = w.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn arb_dominant_routes_arbitrage() {
        let f = fused(true);
        assert_eq!(select_strategy(&f), "arbitrage_capture");
    }

    #[test]
    fn low_liquidity_skips() {
        let cfg = AlphaConfig::from_env();
        let mut f = fused(false);
        f.liquidity_conditions = 0.1;
        assert!(score_to_alpha(&f, &cfg, "w").is_none());
    }
}
