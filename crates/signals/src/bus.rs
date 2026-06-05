//! Signal event bus — a bounded crossbeam channel for `SignalEvent` output.
//!
//! Per `.cursor/rules.md`: "use crossbeam for eventing".
//!
//! Consumers subscribe by holding a `SignalReceiver`.  The `SignalEngine`
//! publishes via `SignalSender`.  Both ends are cheap to clone.

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::types::SignalEvent;

/// Sender half of the signal bus.
pub type SignalSender = Sender<SignalEvent>;

/// Receiver half of the signal bus.
pub type SignalReceiver = Receiver<SignalEvent>;

/// Creates a matched `(SignalSender, SignalReceiver)` pair with the given
/// bounded capacity.
///
/// The capacity should be set to `cfg.signal_channel_capacity`.
/// When the channel is full, `SignalEngine` drops overflow signals rather than
/// blocking (use `try_send` on the sender).
pub fn signal_channel(capacity: usize) -> (SignalSender, SignalReceiver) {
    bounded(capacity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Pubkey;
    use crate::types::{Direction, FeatureVector, SignalEvent, SignalType};

    fn dummy_signal() -> SignalEvent {
        SignalEvent {
            signal_id: 42,
            timestamp_micros: 1_000_000,
            pool_address: Pubkey::new([1; 32]),
            signal_type: SignalType::Momentum,
            strength: 0.8,
            confidence: 0.7,
            direction: Direction::Long,
            timeframe_secs: 60,
            feature_vector: FeatureVector {
                volume_short: 1.0,
                volume_long: 2.0,
                price_velocity: 0.01,
                liquidity_delta_pct: 0.0,
                whale_activity_score: 0.0,
                smart_money_score: 0.0,
                data_points: 5,
            },
            explanation: "test".to_owned(),
        }
    }

    #[test]
    fn send_and_receive_round_trip() {
        let (tx, rx) = signal_channel(4);
        tx.send(dummy_signal()).unwrap();
        let received = rx.recv().unwrap();
        assert_eq!(received.signal_id, 42);
    }

    #[test]
    fn full_channel_try_send_returns_err() {
        let (tx, _rx) = signal_channel(1);
        tx.try_send(dummy_signal()).unwrap();
        // Channel is full — second send must error.
        assert!(tx.try_send(dummy_signal()).is_err());
    }

    #[test]
    fn receiver_disconnects_on_sender_drop() {
        let (tx, rx) = signal_channel(4);
        drop(tx);
        assert!(rx.recv().is_err());
    }
}
