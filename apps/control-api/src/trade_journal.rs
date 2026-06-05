//! Durable in-memory ring buffer of trade lifecycle events for WS replay.

use std::collections::VecDeque;

use crate::dto::TradeEventDto;

/// Maximum trade events retained for reconnect replay.
pub const TRADE_JOURNAL_CAPACITY: usize = 500;

/// Number of recent events replayed to a client on WebSocket connect.
pub const TRADE_REPLAY_COUNT: usize = 100;

#[derive(Debug, Default)]
pub struct TradeJournal {
    events: VecDeque<TradeEventDto>,
}

impl TradeJournal {
    pub fn new() -> Self {
        Self {
            events: VecDeque::with_capacity(TRADE_JOURNAL_CAPACITY.min(64)),
        }
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Append an event, evicting the oldest when at capacity.
    pub fn push(&mut self, event: TradeEventDto) {
        if self.events.len() >= TRADE_JOURNAL_CAPACITY {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Return the last `n` events in chronological order.
    pub fn recent(&self, n: usize) -> Vec<TradeEventDto> {
        let start = self.events.len().saturating_sub(n);
        self.events.iter().skip(start).cloned().collect()
    }

    pub fn all(&self) -> Vec<TradeEventDto> {
        self.events.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{TradeModeDto, TradeSideDto, TradeStageDto};

    fn sample_event(id: &str, ts: u64) -> TradeEventDto {
        TradeEventDto {
            v: TradeEventDto::SCHEMA_VERSION,
            trade_id: id.to_owned(),
            wallet_id: "paper".to_owned(),
            source_strategy: "scalp".to_owned(),
            pair: "SOL/USDC".to_owned(),
            side: TradeSideDto::Long,
            size_usd: 100.0,
            expected_pnl_usd: 1.0,
            tx_signature: None,
            timestamp_us: ts,
            stage: TradeStageDto::Started,
            mode: TradeModeDto::Paper,
            reject_reason: None,
            signal_id: Some(1),
        }
    }

    #[test]
    fn journal_evicts_oldest_at_capacity() {
        let mut journal = TradeJournal::new();
        for i in 0..TRADE_JOURNAL_CAPACITY + 10 {
            journal.push(sample_event(&format!("t{i}"), i as u64));
        }
        assert_eq!(journal.len(), TRADE_JOURNAL_CAPACITY);
        let recent = journal.recent(5);
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].trade_id, format!("t{}", TRADE_JOURNAL_CAPACITY + 5));
    }

    #[test]
    fn recent_returns_chronological_tail() {
        let mut journal = TradeJournal::new();
        journal.push(sample_event("a", 1));
        journal.push(sample_event("b", 2));
        journal.push(sample_event("c", 3));
        let recent = journal.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].trade_id, "b");
        assert_eq!(recent[1].trade_id, "c");
    }
}
