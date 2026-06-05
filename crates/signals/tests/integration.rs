//! Integration tests for the signal engine pipeline.
//!
//! Each test exercises the **complete** pipeline:
//!   SignalInput → FeatureStore → Processors → Aggregator → SignalBus
//!
//! # Requirements (from spec)
//!
//! 1. `whale_flow_test`
//! 2. `smart_money_test`
//! 3. `momentum_test`
//! 4. `noise_rejection_test`
//! 5. `cooldown_behavior_test`
//!
//! All tests are deterministic (synthetic events, fixed timestamps, no RPC).

use common::{MarketEvent, PoolUpdate, Pubkey, SwapEvent};
use config::SignalEngineConfig;
use signals::{Direction, SignalEngine, SignalInput, SignalType, WhaleEvent};

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

fn test_cfg() -> SignalEngineConfig {
    SignalEngineConfig {
        whale_threshold_usd: 10_000.0,
        momentum_window_secs: 60,
        smart_money_min_score: 0.70,
        signal_min_strength: 0.10,
        signal_min_confidence: 0.10,
        // Short cooldown so seed-triggered signals don't block test events.
        cooldown_secs: 5,
        feature_store_max_age_secs: 300,
        signal_channel_capacity: 256,
    }
}

fn pool(b: u8) -> Pubkey {
    Pubkey::new([b; 32])
}

/// Injects `n` swap events into the engine at 2 s intervals, starting at `t0_micros`.
fn seed_swaps(engine: &SignalEngine, p: Pubkey, n: u64, t0_micros: u64, amount: u128) {
    for i in 0..n {
        let evt = MarketEvent::SwapEvent(SwapEvent {
            pool: p,
            input_mint: Pubkey::new([0; 32]),
            output_mint: Pubkey::new([0; 32]),
            amount_in: amount,
            amount_out: amount - 1,
        });
        engine.process_at(SignalInput::Market(evt), t0_micros + i * 2_000_000);
    }
}

/// Injects `n` pool updates with linearly increasing `sqrt_price`.
fn seed_price_ramp(engine: &SignalEngine, p: Pubkey, n: u64, t0_micros: u64) {
    for i in 0..n {
        let evt = MarketEvent::PoolUpdate(PoolUpdate {
            pool: Some(p),
            token_a_mint: None,
            token_b_mint: None,
            liquidity: Some(100_000 + i as u128 * 100),
            sqrt_price: Some(1_000_000 + i as u128 * 10_000),
            fee_rate: None,
        });
        engine.process_at(SignalInput::Market(evt), t0_micros + i * 2_000_000);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. whale_flow_test
// ─────────────────────────────────────────────────────────────────────────────

/// A sufficiently large whale swap must produce a `WhaleFlow` signal with
/// `strength > 0` and `direction == Long`.
#[test]
fn whale_flow_test() {
    let (engine, rx) = SignalEngine::new(test_cfg());
    let p = pool(1);
    let t0: u64 = 10_000_000;

    // Prime the feature store with 10 market events.
    seed_swaps(&engine, p, 10, t0, 500);

    // 5× threshold whale swap.
    let whale = WhaleEvent {
        timestamp_micros: t0 + 20_000_000,
        pool_address: p,
        swap_amount_usd: 50_000.0, // 5× the 10_000 threshold
        direction: Direction::Long,
        profitability_score: 0.6,
    };

    let signals = engine.process_at(SignalInput::Whale(whale), t0 + 20_000_000);

    let whale_signals: Vec<_> = signals
        .iter()
        .filter(|s| s.signal_type == SignalType::WhaleFlow)
        .collect();

    assert!(!whale_signals.is_empty(), "expected at least one WhaleFlow signal");

    let s = &whale_signals[0];
    assert!(s.strength > 0.0, "strength must be positive");
    assert!(s.strength <= 1.0, "strength must not exceed 1.0");
    assert_eq!(s.direction, Direction::Long);
    assert_eq!(s.pool_address, p);

    // Signal must also appear on the bus.
    let bus_signals: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        bus_signals.iter().any(|s| s.signal_type == SignalType::WhaleFlow),
        "WhaleFlow signal must be published to the signal bus"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. smart_money_test
// ─────────────────────────────────────────────────────────────────────────────

/// Repeated high-profitability whale events must trigger a `SmartMoney` signal.
/// The test verifies that below-threshold profitability is rejected, and that
/// a sustained pattern of high-score events produces a signal.
#[test]
fn smart_money_test() {
    let (engine, _rx) = SignalEngine::new(test_cfg());
    let p = pool(2);

    // ── Rejection case ───────────────────────────────────────────────────
    // A below-threshold profitability whale must NOT produce a SmartMoney signal
    // (it may produce WhaleFlow, which is acceptable).
    let low_score_whale = WhaleEvent {
        timestamp_micros: 1_000_000,
        pool_address: p,
        swap_amount_usd: 20_000.0,
        direction: Direction::Long,
        profitability_score: 0.40, // below smart_money_min_score=0.70
    };
    let out = engine.process_at(SignalInput::Whale(low_score_whale), 1_000_000);
    assert!(
        out.iter().all(|s| s.signal_type != SignalType::SmartMoney),
        "low profitability must not produce SmartMoney signal"
    );

    // ── Clustering case ──────────────────────────────────────────────────
    // Inject 3 high-score events spaced > cooldown (5 s) apart to build the
    // aggregate smart_money_score above the threshold.
    // t = 10 s, 16 s, 22 s (each > 5 s from the previous signal emission).
    for i in 0..3u64 {
        let ts = 10_000_000 + i * 6_000_000;
        let smart = WhaleEvent {
            timestamp_micros: ts,
            pool_address: p,
            swap_amount_usd: 25_000.0,
            direction: Direction::Long,
            profitability_score: 0.85,
        };
        engine.process_at(SignalInput::Whale(smart), ts);
    }

    // Trigger event at t = 28 s (> 5 s after the last emission at 22 s).
    let trigger_ts: u64 = 28_000_000;
    let trigger = WhaleEvent {
        timestamp_micros: trigger_ts,
        pool_address: p,
        swap_amount_usd: 30_000.0,
        direction: Direction::Long,
        profitability_score: 0.90,
    };
    let final_out = engine.process_at(SignalInput::Whale(trigger), trigger_ts);

    let sm: Vec<_> = final_out
        .iter()
        .filter(|s| s.signal_type == SignalType::SmartMoney)
        .collect();

    assert!(!sm.is_empty(), "expected SmartMoney signal after sustained high-score events");

    let sig = sm[0];
    assert!(sig.strength >= 0.0 && sig.strength <= 1.0);
    assert!(sig.confidence >= 0.0 && sig.confidence <= 1.0);
    // Explanation must contain structured fields, not free text.
    assert!(sig.explanation.contains("type=SmartMoney"), "explanation missing type tag");
    assert!(sig.explanation.contains("profitability="), "explanation missing profitability");
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. momentum_test
// ─────────────────────────────────────────────────────────────────────────────

/// A sharp increase in swap volume with a rising price must trigger a
/// `Momentum` signal with `direction == Long`.
///
/// Layout (all timestamps in microseconds, long_window = 60 s):
///
/// ```text
/// 0 ──── [historical seeds: 0..25 s] ──── 30 s ──── [short window] ──── 60 s
///                                          ^^^                          ^^^
///                                     short_cutoff              now = 60 s
/// ```
///
/// Seeds go into the HISTORICAL part of the long window (before the short
/// window), so they provide a baseline rate.  The spike enters exclusively
/// inside the short window.  The historical-baseline acceleration then
/// correctly reflects the spike.
#[test]
fn momentum_test() {
    let (engine, rx) = SignalEngine::new(test_cfg());
    let p = pool(3);

    // ── Historical baseline ──────────────────────────────────────────────
    // 10 uniform swaps at [0, 2, 4, …, 18] s (all inside [0, 30) → historical
    // portion of the 60 s long window when now = 60 s).
    let baseline_t0: u64 = 0;
    seed_swaps(&engine, p, 10, baseline_t0, 100);

    // Rising price during baseline period.
    seed_price_ramp(&engine, p, 10, baseline_t0);

    // ── Gap ─────────────────────────────────────────────────────────────
    // Allow any baseline-triggered cooldowns (cooldown_secs=5) to expire.
    // The furthest possible cooldown-setting event is at ~18 s.
    // Cooldown expires at 18 s + 5 s = 23 s.  Spike starts at 31 s > 23 s.

    // ── Spike (inside short window) ──────────────────────────────────────
    // now = 60 s; short_cutoff = 30 s.  Spike from 31 s → 49 s.
    // Historical volume in [0, 30): 10 × 100 = 1 000; historical_rate = 33.3/s.
    // Spike volume in [30, 60]: 10 × 1 000 000 = 10 000 000; recent_rate = 333 333/s.
    // volume_acceleration = 333 333 / 33.3 ≈ 10 000 >> 1.1 → fires.
    let spike_t0: u64 = 31_000_000;
    for i in 0..10u64 {
        let evt = MarketEvent::SwapEvent(SwapEvent {
            pool: p,
            input_mint: Pubkey::new([0; 32]),
            output_mint: Pubkey::new([0; 32]),
            amount_in: 1_000_000,
            amount_out: 999_999,
        });
        engine.process_at(SignalInput::Market(evt), spike_t0 + i * 2_000_000);
    }

    // Final price-up event to confirm positive price_velocity; also triggers
    // the momentum check at now = 60 s.
    let now: u64 = 60_000_000;
    let final_price = MarketEvent::PoolUpdate(PoolUpdate {
        pool: Some(p),
        token_a_mint: None,
        token_b_mint: None,
        liquidity: Some(200_000),
        sqrt_price: Some(3_000_000),  // far above baseline seeds (1_000_000..1_090_000)
        fee_rate: None,
    });
    let signals = engine.process_at(SignalInput::Market(final_price), now);

    let mom: Vec<_> = signals
        .iter()
        .filter(|s| s.signal_type == SignalType::Momentum)
        .collect();

    assert!(!mom.is_empty(), "expected Momentum signal after volume spike");

    let sig = mom[0];
    assert!(sig.strength > 0.0, "strength must be positive");
    assert_eq!(sig.direction, Direction::Long, "rising price must give Long direction");

    // Verify full pipeline: bus must carry the signal too.
    let bus_signals: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        bus_signals.iter().any(|s| s.signal_type == SignalType::Momentum),
        "Momentum signal must appear on the bus"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. noise_rejection_test
// ─────────────────────────────────────────────────────────────────────────────

/// Low-strength and low-confidence events must be suppressed by the aggregator.
/// This validates the threshold gates work end-to-end.
#[test]
fn noise_rejection_test() {
    // Use stricter thresholds to make the noise-rejection surface explicit.
    let strict_cfg = SignalEngineConfig {
        signal_min_strength: 0.50,
        signal_min_confidence: 0.60,
        whale_threshold_usd: 10_000.0,
        momentum_window_secs: 60,
        smart_money_min_score: 0.70,
        cooldown_secs: 30,
        feature_store_max_age_secs: 300,
        signal_channel_capacity: 256,
    };

    let (engine, rx) = SignalEngine::new(strict_cfg);
    let p = pool(4);
    let t0: u64 = 10_000_000;

    // Case A: just-above-threshold whale (1.01×) with no history → low confidence.
    let borderline_whale = WhaleEvent {
        timestamp_micros: t0,
        pool_address: p,
        swap_amount_usd: 10_100.0,  // only 1.01× threshold
        direction: Direction::Long,
        profitability_score: 0.4,
    };
    let out_a = engine.process_at(SignalInput::Whale(borderline_whale), t0);
    // tanh(1.01) ≈ 0.76 passes strength, but confidence = history_factor * 0.6 + score * 0.4
    // with 0 data points → history_factor = 0 → confidence = 0 + 0.4 * whale_score
    // whale_score is also 0 (no prior whale events) → confidence ≈ 0 < 0.60
    for s in &out_a {
        assert!(
            s.signal_type != SignalType::WhaleFlow || s.confidence >= 0.60,
            "WhaleFlow with no history must not pass the 0.60 confidence gate"
        );
    }

    // Case B: tiny whale swap (0.5× threshold).
    let tiny_whale = WhaleEvent {
        timestamp_micros: t0 + 1_000_000,
        pool_address: pool(5),
        swap_amount_usd: 5_000.0,
        direction: Direction::Long,
        profitability_score: 0.9,
    };
    let out_b = engine.process_at(SignalInput::Whale(tiny_whale), t0 + 1_000_000);
    assert!(
        out_b.iter().all(|s| s.signal_type != SignalType::WhaleFlow),
        "whale below threshold must be rejected"
    );

    // Case C: low-acceleration market event (volume flat).
    let flat_pool = pool(6);
    seed_swaps(&engine, flat_pool, 20, t0, 200); // uniform volume — no spike
    let flat_evt = MarketEvent::SwapEvent(SwapEvent {
        pool: flat_pool,
        input_mint: Pubkey::new([0; 32]),
        output_mint: Pubkey::new([0; 32]),
        amount_in: 200,
        amount_out: 199,
    });
    let out_c = engine.process_at(SignalInput::Market(flat_evt), t0 + 60_000_000);
    assert!(
        out_c.iter().all(|s| s.signal_type != SignalType::Momentum),
        "flat volume must not trigger Momentum"
    );

    // Verify nothing leaked to the bus from cases B and C.
    let noise: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let noise_b: Vec<_> = noise
        .iter()
        .filter(|s| s.pool_address == pool(5) && s.signal_type == SignalType::WhaleFlow)
        .collect();
    assert!(noise_b.is_empty(), "sub-threshold whale must not reach the bus");
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. cooldown_behavior_test
// ─────────────────────────────────────────────────────────────────────────────

/// After emitting a signal for a pool, no second signal must be emitted until
/// the configured cooldown period elapses.
#[test]
fn cooldown_behavior_test() {
    let cfg = SignalEngineConfig {
        cooldown_secs: 10,
        signal_min_strength: 0.05,
        signal_min_confidence: 0.05,
        whale_threshold_usd: 5_000.0,
        momentum_window_secs: 60,
        smart_money_min_score: 0.70,
        feature_store_max_age_secs: 300,
        signal_channel_capacity: 256,
    };

    let (engine, rx) = SignalEngine::new(cfg);
    let p = pool(7);
    let t0: u64 = 10_000_000;

    // Seed some history so confidence will be non-trivial.
    seed_swaps(&engine, p, 15, t0, 300);

    let make_whale = |ts: u64| WhaleEvent {
        timestamp_micros: ts,
        pool_address: p,
        swap_amount_usd: 30_000.0,
        direction: Direction::Long,
        profitability_score: 0.6,
    };

    // --- First emission ---
    let first_out = engine.process_at(SignalInput::Whale(make_whale(t0 + 30_000_000)), t0 + 30_000_000);
    let first_whale: Vec<_> = first_out
        .iter()
        .filter(|s| s.signal_type == SignalType::WhaleFlow)
        .collect();
    assert!(!first_whale.is_empty(), "first signal must be emitted");

    // Drain bus.
    while rx.try_recv().is_ok() {}

    // --- Immediate re-trigger (1 s later, inside 10 s cooldown) ---
    let second_out = engine.process_at(
        SignalInput::Whale(make_whale(t0 + 31_000_000)),
        t0 + 31_000_000,
    );
    let second_whale: Vec<_> = second_out
        .iter()
        .filter(|s| s.signal_type == SignalType::WhaleFlow)
        .collect();
    assert!(
        second_whale.is_empty(),
        "signal must be suppressed during cooldown (1 s after first emission)"
    );
    assert!(
        rx.try_recv().is_err(),
        "bus must be empty during cooldown"
    );

    // --- Re-trigger after cooldown expires (11 s later) ---
    let third_out = engine.process_at(
        SignalInput::Whale(make_whale(t0 + 41_000_000)),
        t0 + 41_000_000,
    );
    let third_whale: Vec<_> = third_out
        .iter()
        .filter(|s| s.signal_type == SignalType::WhaleFlow)
        .collect();
    assert!(
        !third_whale.is_empty(),
        "signal must be re-emitted after cooldown expires (11 s after first)"
    );
    assert!(
        rx.try_recv().is_ok(),
        "bus must carry the post-cooldown signal"
    );
}
