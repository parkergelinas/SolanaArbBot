//! Smart-money signal detector.
//!
//! Emits a `SmartMoney` signal when wallet profitability clustering is detected
//! on a pool.  "Smart money" is defined purely by the caller-supplied
//! `profitability_score` — no hardcoded wallet lists are used.
//!
//! # Criteria (all config-driven)
//!
//! * `profitability_score >= cfg.smart_money_min_score`
//! * `smart_money_score` from feature store reflects a pattern of recurring
//!   high-profitability activity (not a single lucky trade)
//! * Strength = smart_money_score (already normalised to [0, 1])
//! * Confidence = min(whale_event_count / 3, 1.0) — requires multiple events
//!   to avoid noise from one-off swaps

use config::SignalEngineConfig;

use crate::feature_store::ComputedFeatures;
use crate::processor::SignalProcessor;
use crate::types::{
    FeatureVector, SignalEvent, SignalInput, SignalType, WhaleEvent,
    fnv1a, short_addr,
};

pub struct SmartMoneyProcessor;

impl SignalProcessor for SmartMoneyProcessor {
    fn name(&self) -> &'static str {
        "SmartMoneyProcessor"
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

        // The triggering event must itself qualify as smart money.
        if whale.profitability_score < cfg.smart_money_min_score {
            return vec![];
        }

        // The aggregate score from the feature store must be above threshold too.
        // This prevents a single high-scoring event from triggering a signal without
        // historical behavioural clustering.
        if features.smart_money_score < cfg.smart_money_min_score {
            return vec![];
        }

        let strength = features.smart_money_score.clamp(0.0, 1.0);

        // Require at least 2 qualifying events for non-trivial confidence.
        let confidence =
            f64::min(features.whale_event_count as f64 / 2.0, 1.0).clamp(0.0, 1.0);

        // Direction mirrors the triggering whale event; smart money is acting
        // with intent, so the most recent direction is the relevant signal.
        let direction = whale.direction;

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
             profitability={:.3} min_score={:.3} smart_money_score={:.3} \
             whale_events={} vol_short={:.2} vol_long={:.2}",
            SignalType::SmartMoney,
            short_addr(whale.pool_address),
            direction,
            strength,
            confidence,
            whale.profitability_score,
            cfg.smart_money_min_score,
            features.smart_money_score,
            features.whale_event_count,
            features.volume_short,
            features.volume_long,
        );

        let signal_id = fnv1a(whale.pool_address, SignalType::SmartMoney as u8, now_micros);

        vec![SignalEvent {
            signal_id,
            timestamp_micros: now_micros,
            pool_address: whale.pool_address,
            signal_type: SignalType::SmartMoney,
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
    use common::Pubkey;
    use config::SignalEngineConfig;
    use crate::feature_store::ComputedFeatures;
    use crate::types::{Direction, WhaleEvent};

    fn pool() -> Pubkey { Pubkey::new([9; 32]) }
    fn cfg() -> SignalEngineConfig { SignalEngineConfig::default() }

    fn whale_input(profitability: f64) -> SignalInput {
        SignalInput::Whale(WhaleEvent {
            timestamp_micros: 1_000_000,
            pool_address: pool(),
            swap_amount_usd: 20_000.0,
            direction: Direction::Long,
            profitability_score: profitability,
        })
    }

    fn features(whale_count: usize, smart_score: f64) -> ComputedFeatures {
        ComputedFeatures {
            pool: pool(),
            volume_short: 200.0,
            volume_long: 1_000.0,
            price_velocity: 0.005,
            liquidity_delta_pct: 0.01,
            whale_activity_score: 0.4,
            smart_money_score: smart_score,
            last_price: 2_000.0,
            last_liquidity: 100_000.0,
            whale_event_count: whale_count,
            data_points: 20,
        }
    }

    #[test]
    fn below_min_score_returns_empty() {
        let input = whale_input(0.3); // below default 0.70
        let out = SmartMoneyProcessor.process(&input, &features(5, 0.8), &cfg(), 1_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn low_aggregate_score_suppressed() {
        let input = whale_input(0.85); // triggering event qualifies
        // but historical smart_money_score is below threshold
        let out = SmartMoneyProcessor.process(&input, &features(5, 0.3), &cfg(), 1_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn emits_when_both_scores_qualify() {
        let input = whale_input(0.85);
        let out = SmartMoneyProcessor.process(&input, &features(4, 0.82), &cfg(), 2_000_000);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].signal_type, SignalType::SmartMoney);
    }

    #[test]
    fn more_events_gives_higher_confidence() {
        let input = whale_input(0.9);
        let few = SmartMoneyProcessor.process(&input, &features(1, 0.85), &cfg(), 3_000_000);
        let many = SmartMoneyProcessor.process(&input, &features(8, 0.85), &cfg(), 3_000_000);
        assert!(many[0].confidence >= few[0].confidence);
    }

    #[test]
    fn market_event_returns_empty() {
        use common::{MarketEvent, SwapEvent};
        let p = pool();
        let input = SignalInput::Market(MarketEvent::SwapEvent(SwapEvent {
            pool: p,
            input_mint: Pubkey::new([0; 32]),
            output_mint: Pubkey::new([0; 32]),
            amount_in: 500,
            amount_out: 498,
        }));
        let out = SmartMoneyProcessor.process(&input, &features(5, 0.9), &cfg(), 4_000_000);
        assert!(out.is_empty());
    }

    #[test]
    fn confidence_capped_at_one() {
        let input = whale_input(0.95);
        let out = SmartMoneyProcessor.process(&input, &features(100, 0.95), &cfg(), 5_000_000);
        assert!(out[0].confidence <= 1.0);
    }
}
