use std::sync::Arc;

use crate::{AccountKey, ProgramId, SignatureBytes, Slot, UnixNanos};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventSource {
    AccountStream,
    LogStream,
    TransactionStream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventMeta {
    pub slot: Slot,
    pub source: EventSource,
    pub received_at_unix_nanos: UnixNanos,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketEvent {
    PoolUpdate(PoolUpdate),
    SwapEvent(SwapEvent),
    TickUpdate(TickUpdate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolUpdate {
    pub meta: EventMeta,
    pub pool: AccountKey,
    pub program: ProgramId,
    pub account_data: Arc<[u8]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwapEvent {
    pub meta: EventMeta,
    pub signature: SignatureBytes,
    pub pool: Option<AccountKey>,
    pub log: Arc<str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TickUpdate {
    pub meta: EventMeta,
    pub pool: AccountKey,
    pub tick_array: AccountKey,
    pub program: ProgramId,
    pub account_data: Arc<[u8]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_event_clone_reuses_payload_allocation() {
        let payload: Arc<[u8]> = Arc::from([1_u8, 2, 3]);
        let event = MarketEvent::PoolUpdate(PoolUpdate {
            meta: EventMeta {
                slot: 42,
                source: EventSource::AccountStream,
                received_at_unix_nanos: 7,
            },
            pool: [1; 32],
            program: [2; 32],
            account_data: Arc::clone(&payload),
        });

        let cloned = event.clone();

        match cloned {
            MarketEvent::PoolUpdate(update) => {
                assert_eq!(update.account_data.as_ref(), &[1, 2, 3]);
                assert_eq!(Arc::strong_count(&payload), 3);
            }
            _ => panic!("unexpected event variant"),
        }
    }
}
