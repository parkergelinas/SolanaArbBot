//! Signal-strength-scaled position sizing model.

use config::ScalerConfig;
use signals::SignalEvent;

/// Compute the USD position size for a trade candidate.
///
/// Formula: `size = base_size × strength^k × confidence`
///
/// Where:
/// - `base_size = capital_usd × cfg.base_position_pct`
/// - `k = cfg.strength_scaling_exponent`
///
/// The result is clamped to `[cfg.min_position_usd, cfg.max_position_usd]`.
///
/// # Examples
///
/// A 0.7-strength, 0.9-confidence signal with default config and $10,000
/// capital produces a position near $120 (before clamping).
pub fn compute_size(signal: &SignalEvent, capital_usd: f64, cfg: &ScalerConfig) -> f64 {
    let base_size = capital_usd * cfg.base_position_pct;
    let scaled = base_size
        * signal.strength.powf(cfg.strength_scaling_exponent)
        * signal.confidence;
    scaled.clamp(cfg.min_position_usd, cfg.max_position_usd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Pubkey;
    use config::ScalerConfig;
    use signals::{Direction, FeatureVector, SignalEvent, SignalType};

    fn make_signal(strength: f64, confidence: f64) -> SignalEvent {
        SignalEvent {
            signal_id: 1,
            timestamp_micros: 1_000_000,
            pool_address: Pubkey::new([1; 32]),
            signal_type: SignalType::Momentum,
            strength,
            confidence,
            direction: Direction::Long,
            timeframe_secs: 60,
            feature_vector: FeatureVector {
                volume_short: 0.0,
                volume_long: 0.0,
                price_velocity: 0.0,
                liquidity_delta_pct: 0.0,
                whale_activity_score: 0.0,
                smart_money_score: 0.0,
                data_points: 0,
            },
            explanation: String::new(),
        }
    }

    #[test]
    fn size_clamps_at_minimum() {
        let cfg = ScalerConfig {
            min_position_usd: 100.0,
            max_position_usd: 5_000.0,
            base_position_pct: 0.02,
            strength_scaling_exponent: 1.5,
            ..ScalerConfig::default()
        };
        // Near-zero strength → clamp to minimum.
        let size = compute_size(&make_signal(0.01, 0.01), 1_000.0, &cfg);
        assert_eq!(size, cfg.min_position_usd);
    }

    #[test]
    fn size_clamps_at_maximum() {
        let cfg = ScalerConfig {
            min_position_usd: 100.0,
            max_position_usd: 5_000.0,
            base_position_pct: 0.02,
            strength_scaling_exponent: 1.5,
            ..ScalerConfig::default()
        };
        // Maximum strength+confidence with large capital → clamp to maximum.
        let size = compute_size(&make_signal(1.0, 1.0), 10_000_000.0, &cfg);
        assert_eq!(size, cfg.max_position_usd);
    }

    #[test]
    fn higher_strength_produces_larger_size() {
        let cfg = ScalerConfig::default();
        let capital = 100_000.0;
        let low = compute_size(&make_signal(0.4, 0.8), capital, &cfg);
        let high = compute_size(&make_signal(0.9, 0.8), capital, &cfg);
        assert!(high > low, "higher strength should yield larger size");
    }

    #[test]
    fn size_is_deterministic() {
        let cfg = ScalerConfig::default();
        let s = make_signal(0.7, 0.85);
        assert_eq!(
            compute_size(&s, 50_000.0, &cfg),
            compute_size(&s, 50_000.0, &cfg)
        );
    }
}
