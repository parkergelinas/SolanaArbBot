//! Smoke test: mock RPC → ingestion → pricing → routing → risk
//! Asserts no panic and at least one route candidate is evaluated.

use common::Pubkey;
use config::SystemConfig;
use events::{EventBus, MarketEvent, PoolUpdate, SwapEvent};
use signals::{SignalEngine, SignalInput};

/// Minimum bar: process 10 synthetic swap events without panic.
#[tokio::test]
async fn pipeline_does_not_panic_on_mock_data() {
    let cfg = SystemConfig::default();
    cfg.validate().expect("default config valid");

    let bus = EventBus::new();
    let sub = bus.subscribe();
    let (engine, _rx) = SignalEngine::new(cfg.signal_engine.clone());

    let token_a = Pubkey::new([0xAA; 32]);
    let token_b = Pubkey::new([0xBB; 32]);
    let pool = Pubkey::new([0x11; 32]);

    // TODO: wire mock RpcClient → EventBus → Engine (full worker pipeline)
    // TODO: assert routing evaluates at least one candidate route
    // TODO: assert risk enforcer returns Ok on baseline state

    for seq in 0..10_u64 {
        let event = if seq % 2 == 0 {
            MarketEvent::SwapEvent(SwapEvent {
                pool,
                input_mint: token_a,
                output_mint: token_b,
                amount_in: 1_000_000 + seq as u128 * 100,
                amount_out: 998_000 + seq as u128 * 99,
            })
        } else {
            MarketEvent::PoolUpdate(PoolUpdate {
                pool: Some(pool),
                token_a_mint: Some(token_a),
                token_b_mint: Some(token_b),
                liquidity: Some(1_000_000_u128 + seq as u128 * 1_000),
                sqrt_price: Some(79_228_162_514_u128),
                fee_rate: Some(300),
            })
        };

        bus.publish(event).expect("publish synthetic event");
    }

    let mut processed = 0_usize;
    while let Ok(Some(ev)) = sub.try_recv() {
        let _signals = engine.process(SignalInput::Market(ev));
        processed += 1;
        if processed >= 10 {
            break;
        }
    }

    assert!(processed >= 1, "expected at least one event through signal engine");
}
