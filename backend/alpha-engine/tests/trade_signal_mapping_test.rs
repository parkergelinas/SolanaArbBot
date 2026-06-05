//! AlphaSignal → TradeSignal mapping test.

use alpha_engine::types::{AlphaSignal, TradeSignal};

#[test]
fn alpha_to_trade_signal_mapping() {
    let alpha = AlphaSignal {
        signal_id: "test".into(),
        token_in: "USDC".into(),
        token_out: "SOL".into(),
        wallet: "whale_1".into(),
        confidence: 0.85,
        expected_edge: 12.0,
        size_usd: 150.0,
        strategy: "momentum_follow".into(),
        score: 0.85,
        direction: "long".into(),
        timestamp: 1_000,
    };

    let trade = TradeSignal::from_alpha(&alpha);
    assert_eq!(trade.token, "SOL");
    assert_eq!(trade.direction, "long");
    assert_eq!(trade.strategy, "momentum_follow");
    assert!((trade.confidence - 0.85).abs() < f64::EPSILON);
}
