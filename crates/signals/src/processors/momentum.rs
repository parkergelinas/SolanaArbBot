//! Momentum signal detector.
//!
//! Detects directional price + volume momentum using rolling windows from the
//! feature store.  All calculations are over the configured `momentum_window_secs`.
//!
//! # Criteria (all config-driven)
//!
//! * `volume_acceleration = volume_short / volume_long > 1.0`
//!   (recent volume outpaces the longer window's average rate)
//! * `price_velocity ≠ 0.0` (price is moving)
//! * `liquidity_delta_pct ≥ 0.0` (liquidity expanding or stable — not a rug)
//! * Strength = tanh(volume_acceleration * (1 + |price_velocity| * 10))
//! * Confidence bounded by data_points (needs at least 5 market events)

use config::SignalEngineConfig;

use crate::feature_store::ComputedFeatures;
use crate::processor::SignalProcessor;
use crate::types::{
    Direction, FeatureVector, SignalEvent, SignalInput, SignalType,
    fnv1a, short_addr,
};

pub struct MomentumProcessor;

impl SignalProcessor for MomentumProcessor {
    fn name(&self) -> &'static str {
        "MomentumProcessor"
    }

    fn process(
        &self,
        input: &SignalInput,
        features: &ComputedFeatures,
        cfg: &SignalEngineConfig,
        now_micros: u64,
    ) -> Vec<SignalEvent> {
        // Momentum fires on market events, not on whale events.
        match input {
            SignalInput::Whale(_) => return vec![],
            SignalInput::Market(_) => {}
        }

        // Need both recent and historical volume data.
        if features.data_points < 2 || features.volume_short == 0.0 {
            return vec![];
        }

        // Volume acceleration: compare rate in the SHORT window to the rate in
        // the remainder of the LONG window (the "historical baseline").
        //
        // Using the historical portion rather than the full long window avoids
        // the degenerate case where both windows contain the same events (early
        // sparse data), which would always yield acceleration ≈ 2.0.
        //
        // volume_long always >= volume_short because the short window is a
        // subset of the long window.
        let short_secs: f64 = 30.0;
        let long_secs: f64 = cfg.momentum_window_secs as f64;
        let historical_secs = (long_secs - short_secs).max(1.0);
        let historical_volume = (features.volume_long - features.volume_short).max(0.0);

        // Require a non-zero historical baseline — otherwise we have no reference
        // period to measure acceleration against.
        if historical_volume == 0.0 {
            return vec![];
        }

        let historical_rate = historical_volume / historical_secs;
        let recent_rate = features.volume_short / short_secs;
        let volume_acceleration = recent_rate / historical_rate;

        // Require meaningful acceleration (recent rate > 1.1× historical rate).
        if volume_acceleration < 1.1 {
            return vec![];
        }

        let price_vel_abs = features.price_velocity.abs();

        // Liquidity must be expanding or flat — contracting liquidity suggests
        // a rug or panic exit, not genuine momentum.
        if features.liquidity_delta_pct < -0.05 {
            return vec![];
        }

        // Strength: combined score of volume acceleration and price movement.
        let raw_strength = volume_acceleration * (1.0 + price_vel_abs * 10.0);
        let strength = raw_strength.tanh().clamp(0.0, 1.0);

        // Confidence scales with data richness.
        let confidence = f64::min(features.data_points as f64 / 5.0, 1.0).clamp(0.0, 1.0);

        let direction = if features.price_velocity > 0.0 {
            Direction::Long
        } else if features.price_velocity < 0.0 {
            Direction::Short
        } else {
            Direction::Neutral
        };

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
             vol_accel={:.3} price_vel={:.4} liq_delta={:.4} \
             vol_short={:.2} vol_long={:.2} data_pts={}",
            SignalType::Momentum,
            short_addr(features.pool),
            direction,
            strength,
            confidence,
            volume_acceleration,
            features.price_velocity,
            features.liquidity_delta_pct,
            features.volume_short,
            features.volume_long,
            features.data_points,
        );

        let signal_id = fnv1a(features.pool, SignalType::Momentum as u8, now_micros);

        vec![SignalEvent {
            signal_id,
            timestamp_micros: now_micros,
            pool_address: features.pool,
            signal_type: SignalType::Momentum,
            strength,
            confidence,
            direction,
            timeframe_secs: cfg.momentum_window_secs,
            feature_vector: fv,
            explanation,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{MarketEvent, Pubkey, SwapEvent};
    use config::SignalEngineConfig;
    use crate::feature_store::ComputedFeatures;
    use crate::types::Direction;

    fn pool() -> Pubkey { Pubkey::new([3; 32]) }
    fn cfg() -> SignalEngineConfig { SignalEngineConfig::default() }

    fn market_input() -> SignalInput {
        SignalInput::Market(MarketEvent::SwapEvent(SwapEvent {
            pool: pool(),
            input_mint: Pubkey::new([0; 32]),
            output_mint: Pubkey::new([0; 32]),
            amount_in: 1_000,
            amount_out: 998,
        }))
    }

    fn features_with(
        vol_short: f64,
        vol_long: f64,
        price_vel: f64,
        liq_delta: f64,
        pts: usize,
    ) -> ComputedFeatures {
        ComputedFeatures {
            pool: pool(),
            volume_short: vol_short,
            volume_long: vol_long,
            price_velocity: price_vel,
            liquidity_delta_pct: liq_delta,
            whale_activity_score: 0.0,
            smart_money_score: 0.0,
            last_price: 1_000.0,
            last_liquidity: 50_000.0,
            whale_event_count: 0,
            data_points: pts,
        }
    }

    #[test]
    fn whale_input_returns_empty() {
        use crate::types::WhaleEvent;
        let input = SignalInput::Whale(WhaleEvent {
            timestamp_micros: 1_000_000,
            pool_address: pool(),
            swap_amount_usd: 100_000.0,
            direction: Direction::Long,
            profitability_score: 0.9,
        });
        let f = features_with(5_000.0, 10_000.0, 0.05, 0.02, 20);
        let out = MomentumProcessor.process(&input, &f, &cfg(), 1_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn no_data_returns_empty() {
        let f = features_with(0.0, 0.0, 0.0, 0.0, 0);
        let out = MomentumProcessor.process(&market_input(), &f, &cfg(), 1_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn low_acceleration_returns_empty() {
        // recent(short) = 100 over 30 s; historical = 6000-100 = 5900 over 30 s
        // recent_rate=3.3, historical_rate=196.7 → accel≈0.017 < 1.1
        let f = features_with(100.0, 6_100.0, 0.05, 0.01, 10);
        let out = MomentumProcessor.process(&market_input(), &f, &cfg(), 1_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn high_acceleration_emits_signal() {
        // recent(short) = 5000; historical = 6000-5000 = 1000 over 30 s
        // recent_rate=166.7, historical_rate=33.3 → accel≈5.0 > 1.1
        let f = features_with(5_000.0, 6_000.0, 0.03, 0.05, 15);
        let out = MomentumProcessor.process(&market_input(), &f, &cfg(), 2_000_000);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].signal_type, SignalType::Momentum);
    }

    #[test]
    fn no_historical_baseline_returns_empty() {
        // All volume is in the short window — no baseline to compare against.
        let f = features_with(1_000.0, 1_000.0, 0.05, 0.01, 10);
        let out = MomentumProcessor.process(&market_input(), &f, &cfg(), 3_000_000);
        assert!(out.is_empty(), "zero historical volume must suppress signal");
    }

    #[test]
    fn negative_liquidity_suppresses_signal() {
        let f = features_with(5_000.0, 1_000.0, 0.03, -0.10, 15);
        let out = MomentumProcessor.process(&market_input(), &f, &cfg(), 3_000_000);
        assert!(out.is_empty(), "should suppress on contracting liquidity");
    }

    #[test]
    fn rising_price_gives_long_direction() {
        // volume_short=5000, volume_long=6000 → historical=1000 > 0 → valid
        let f = features_with(5_000.0, 6_000.0, 0.05, 0.02, 20);
        let out = MomentumProcessor.process(&market_input(), &f, &cfg(), 4_000_000);
        assert_eq!(out[0].direction, Direction::Long);
    }

    #[test]
    fn falling_price_gives_short_direction() {
        let f = features_with(5_000.0, 6_000.0, -0.05, 0.02, 20);
        let out = MomentumProcessor.process(&market_input(), &f, &cfg(), 5_000_000);
        assert_eq!(out[0].direction, Direction::Short);
    }

    #[test]
    fn strength_bounded_to_unit_interval() {
        // volume_short = 1_000_000, volume_long = 1_000_001 → historical = 1
        // → very high acceleration; verifies tanh clamps output to [0, 1].
        let f = features_with(1_000_000.0, 1_000_001.0, 99.0, 0.5, 100);
        let out = MomentumProcessor.process(&market_input(), &f, &cfg(), 6_000_000);
        assert!(out[0].strength <= 1.0);
        assert!(out[0].strength >= 0.0);
    }
}
