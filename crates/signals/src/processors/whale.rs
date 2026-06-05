//! Whale-flow signal detector.
//!
//! Emits a `WhaleFlow` signal when a `WhaleEvent` arrives and the feature store
//! confirms significant whale pressure on the pool.
//!
//! # Criteria (all config-driven)
//!
//! * `swap_amount_usd >= cfg.whale_threshold_usd`
//! * `whale_activity_score` from feature store reflects accumulation history
//! * Strength = tanh(swap_amount_usd / whale_threshold_usd)
//! * Confidence bounded by `data_points` saturation

use config::SignalEngineConfig;

use crate::feature_store::ComputedFeatures;
use crate::processor::SignalProcessor;
use crate::types::{
    FeatureVector, SignalEvent, SignalInput, SignalType, WhaleEvent,
    fnv1a, short_addr,
};

pub struct WhaleFlowProcessor;

impl SignalProcessor for WhaleFlowProcessor {
    fn name(&self) -> &'static str {
        "WhaleFlowProcessor"
    }

    fn process(
        &self,
        input: &SignalInput,
        features: &ComputedFeatures,
        cfg: &SignalEngineConfig,
        now_micros: u64,
    ) -> Vec<SignalEvent> {
        let whale: &WhaleEvent = match input {
            SignalInput::Whale(w) => w,
            SignalInput::Market(_) => return vec![],
        };

        if whale.swap_amount_usd < cfg.whale_threshold_usd {
            return vec![];
        }

        let raw_ratio = whale.swap_amount_usd / cfg.whale_threshold_usd;
        let strength = raw_ratio.tanh().clamp(0.0, 1.0);

        // Confidence scales with how much historical context we have.
        // data_points >= 10 → full confidence; < 10 → proportional.
        let history_factor = f64::min(features.data_points as f64 / 10.0, 1.0);
        // Also incorporate the ongoing whale activity score.
        let confidence = (history_factor * 0.6 + features.whale_activity_score * 0.4)
            .clamp(0.0, 1.0);

        let fv = FeatureVector {
            volume_short: features.volume_short,
            volume_long: features.volume_long,
            price_velocity: features.price_velocity,
            liquidity_delta_pct: features.liquidity_delta_pct,
            whale_activity_score: features.whale_activity_score,
            smart_money_score: features.smart_money_score,
            data_points: features.data_points,
        };

        let explanation = format!(
            "type={} pool={} dir={} strength={:.3} confidence={:.3} \
             swap_usd={:.2} threshold_usd={:.2} whale_score={:.3} \
             hist_pts={} vol_short={:.2} vol_long={:.2}",
            SignalType::WhaleFlow,
            short_addr(whale.pool_address),
            whale.direction,
            strength,
            confidence,
            whale.swap_amount_usd,
            cfg.whale_threshold_usd,
            features.whale_activity_score,
            features.data_points,
            features.volume_short,
            features.volume_long,
        );

        let signal_id = fnv1a(whale.pool_address, SignalType::WhaleFlow as u8, now_micros);

        vec![SignalEvent {
            signal_id,
            timestamp_micros: now_micros,
            pool_address: whale.pool_address,
            signal_type: SignalType::WhaleFlow,
            strength,
            confidence,
            direction: whale.direction,
            timeframe_secs: cfg.momentum_window_secs,
            feature_vector: fv,
            explanation,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Pubkey;
    use config::SignalEngineConfig;
    use crate::feature_store::ComputedFeatures;
    use crate::types::{Direction, WhaleEvent};

    fn pool() -> Pubkey { Pubkey::new([5; 32]) }
    fn cfg() -> SignalEngineConfig { SignalEngineConfig::default() }

    fn whale(amount_usd: f64, score: f64) -> SignalInput {
        SignalInput::Whale(WhaleEvent {
            timestamp_micros: 1_000_000,
            pool_address: pool(),
            swap_amount_usd: amount_usd,
            direction: Direction::Long,
            profitability_score: score,
        })
    }

    fn features(data_points: usize, whale_score: f64) -> ComputedFeatures {
        ComputedFeatures {
            pool: pool(),
            volume_short: 100.0,
            volume_long: 500.0,
            price_velocity: 0.01,
            liquidity_delta_pct: 0.02,
            whale_activity_score: whale_score,
            smart_money_score: 0.0,
            last_price: 1_000.0,
            last_liquidity: 50_000.0,
            whale_event_count: 1,
            data_points,
        }
    }

    #[test]
    fn market_event_returns_empty() {
        use common::{MarketEvent, SwapEvent};
        let p = pool();
        let input = SignalInput::Market(MarketEvent::SwapEvent(SwapEvent {
            pool: p,
            input_mint: Pubkey::new([0; 32]),
            output_mint: Pubkey::new([0; 32]),
            amount_in: 1_000,
            amount_out: 999,
        }));
        let out = WhaleFlowProcessor.process(&input, &features(10, 0.5), &cfg(), 2_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn below_threshold_returns_empty() {
        let input = whale(cfg().whale_threshold_usd * 0.5, 0.8);
        let out = WhaleFlowProcessor.process(&input, &features(10, 0.3), &cfg(), 2_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn at_threshold_emits_signal() {
        let input = whale(cfg().whale_threshold_usd, 0.9);
        let out = WhaleFlowProcessor.process(&input, &features(15, 0.5), &cfg(), 2_000_000);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].signal_type, SignalType::WhaleFlow);
        assert_eq!(out[0].direction, Direction::Long);
    }

    #[test]
    fn larger_amount_gives_higher_strength() {
        let f = features(20, 0.6);
        let small = WhaleFlowProcessor.process(&whale(20_000.0, 0.5), &f, &cfg(), 1_000_000);
        let large = WhaleFlowProcessor.process(&whale(100_000.0, 0.5), &f, &cfg(), 1_000_000);
        assert!(large[0].strength > small[0].strength);
    }

    #[test]
    fn strength_is_bounded_to_unit_interval() {
        let f = features(50, 0.9);
        let input = whale(1_000_000.0, 1.0); // huge amount
        let out = WhaleFlowProcessor.process(&input, &f, &cfg(), 3_000_000);
        assert!(out[0].strength <= 1.0);
        assert!(out[0].strength >= 0.0);
    }

    #[test]
    fn signal_id_is_deterministic() {
        let f = features(10, 0.5);
        let input = whale(50_000.0, 0.8);
        let a = WhaleFlowProcessor.process(&input, &f, &cfg(), 5_000_000);
        let b = WhaleFlowProcessor.process(&input, &f, &cfg(), 5_000_000);
        assert_eq!(a[0].signal_id, b[0].signal_id);
    }
}
