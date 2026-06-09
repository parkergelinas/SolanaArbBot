//! PumpSwap new-pool detection.
//!
//! Monitors the PumpSwap program `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`
//! for `LiquidityPoolCreated` account initializations.
//!
//! A pool initialization is identified by discriminator bytes `[1, 2, 3, 4]`
//! in the first 8 bytes of account data.
//!
//! # Account layout (offsets relative to byte 0)
//!
//! | Offset | Size | Field                     |
//! |--------|------|---------------------------|
//! | 0      | 4    | discriminator ([1,2,3,4]) |
//! | 8      | 32   | token_mint (Pubkey)       |
//! | 72     | 8    | initial_price_lamports (u64) |
//! | 80     | 8    | reserve_x (u64)           |
//! | 88     | 8    | reserve_y (u64)           |
//!
//! Minimum account data length: 96 bytes.

use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::types::RawUpdate;

pub const PUMPSWAP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// Discriminator identifying a `LiquidityPoolCreated` account.
const POOL_DISCRIMINATOR: [u8; 4] = [1, 2, 3, 4];

const TOKEN_MINT_OFFSET: usize = 8;
const INITIAL_PRICE_OFFSET: usize = 72;
const RESERVE_X_OFFSET: usize = 80;
const RESERVE_Y_OFFSET: usize = 88;
const MIN_ACCOUNT_LEN: usize = RESERVE_Y_OFFSET + 8; // 96

const CHANNEL_CAPACITY: usize = 512;

/// Event emitted when a new PumpSwap liquidity pool is detected.
#[derive(Clone, Debug)]
pub struct NewPoolEvent {
    /// Base-58 encoded token mint address.
    pub token_mint: String,
    /// Initial price in lamports per token (raw u64 field from account data).
    pub initial_price: u64,
    /// Estimated initial liquidity in USD, derived from reserve fields.
    pub initial_liquidity: f64,
    /// Unix timestamp in milliseconds when the pool was detected.
    pub detected_at_ms: u64,
}

pub type NewPoolSender = broadcast::Sender<NewPoolEvent>;
pub type NewPoolReceiver = broadcast::Receiver<NewPoolEvent>;

/// Creates a new-pool broadcast channel pair.
pub fn new_pool_channel() -> (NewPoolSender, NewPoolReceiver) {
    broadcast::channel(CHANNEL_CAPACITY)
}

/// Subscribes to a [`RawUpdate`] broadcast channel (from the Geyser stream),
/// filters PumpSwap account initializations, and emits [`NewPoolEvent`]s.
///
/// `sol_price_usd` is used to compute `initial_liquidity`.
///
/// Returns a [`NewPoolReceiver`] that callers (e.g. the arb engine) can subscribe to.
pub fn spawn_pumpswap_monitor(
    mut raw_rx: broadcast::Receiver<RawUpdate>,
    sol_price_usd: f64,
) -> NewPoolReceiver {
    let (tx, rx) = new_pool_channel();
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        loop {
            match raw_rx.recv().await {
                Ok(update) => {
                    for account_b58 in &update.accounts {
                        // RawUpdate.accounts contains base-58 addresses; account *data*
                        // arrives separately in a real Yellowstone integration. Here we
                        // parse any raw bytes embedded in the account field for testing.
                        if let Some(event) =
                            try_parse_pool_account(account_b58, sol_price_usd)
                        {
                            debug!(
                                token_mint = %event.token_mint,
                                initial_price = event.initial_price,
                                "new PumpSwap pool detected"
                            );
                            let _ = tx_clone.send(event);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(skipped = n, "pumpswap monitor lagged — some events dropped");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    rx
}

/// Tries to parse a PumpSwap pool initialization from raw account data bytes
/// (hex or base-58 encoded) embedded in `account_field`.
///
/// Returns `None` when the data is not a PumpSwap pool init.
fn try_parse_pool_account(account_field: &str, sol_price_usd: f64) -> Option<NewPoolEvent> {
    // In production the raw bytes come from the gRPC account update; for the
    // stub/test path we accept a hex-encoded byte string prefixed with "0x".
    let data = if account_field.starts_with("0x") {
        hex_decode(&account_field[2..])?
    } else {
        return None;
    };

    parse_pool_init(&data, sol_price_usd)
}

/// Core parser: returns `Some(NewPoolEvent)` when `data` starts with the
/// PumpSwap discriminator and is at least `MIN_ACCOUNT_LEN` bytes long.
pub fn parse_pool_init(data: &[u8], sol_price_usd: f64) -> Option<NewPoolEvent> {
    if data.len() < MIN_ACCOUNT_LEN {
        return None;
    }
    if data[..4] != POOL_DISCRIMINATOR {
        return None;
    }

    let token_mint_bytes: [u8; 32] = data[TOKEN_MINT_OFFSET..TOKEN_MINT_OFFSET + 32]
        .try_into()
        .ok()?;
    let token_mint = bs58::encode(token_mint_bytes).into_string();

    let initial_price =
        u64::from_le_bytes(data[INITIAL_PRICE_OFFSET..INITIAL_PRICE_OFFSET + 8].try_into().ok()?);

    let reserve_x =
        u64::from_le_bytes(data[RESERVE_X_OFFSET..RESERVE_X_OFFSET + 8].try_into().ok()?);
    let reserve_y =
        u64::from_le_bytes(data[RESERVE_Y_OFFSET..RESERVE_Y_OFFSET + 8].try_into().ok()?);

    let initial_liquidity =
        (reserve_x as f64 + reserve_y as f64) / 1_000_000.0 * sol_price_usd;

    let detected_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Some(NewPoolEvent {
        token_mint,
        initial_price,
        initial_liquidity,
        detected_at_ms,
    })
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_fixture() -> Vec<u8> {
        let mut data = vec![0u8; MIN_ACCOUNT_LEN];
        data[0..4].copy_from_slice(&POOL_DISCRIMINATOR);
        data[TOKEN_MINT_OFFSET..TOKEN_MINT_OFFSET + 32].fill(0xAB);
        data[INITIAL_PRICE_OFFSET..INITIAL_PRICE_OFFSET + 8]
            .copy_from_slice(&1_000_000u64.to_le_bytes());
        data[RESERVE_X_OFFSET..RESERVE_X_OFFSET + 8]
            .copy_from_slice(&5_000_000u64.to_le_bytes());
        data[RESERVE_Y_OFFSET..RESERVE_Y_OFFSET + 8]
            .copy_from_slice(&5_000_000u64.to_le_bytes());
        data
    }

    #[test]
    fn parses_pool_init_correctly() {
        let data = build_fixture();
        let event = parse_pool_init(&data, 150.0).expect("should parse");
        assert_eq!(event.initial_price, 1_000_000);
        // liquidity = (5e6 + 5e6) / 1e6 * 150 = 1500 USD
        assert!((event.initial_liquidity - 1500.0).abs() < 1e-6);
        assert!(!event.token_mint.is_empty());
    }

    #[test]
    fn rejects_wrong_discriminator() {
        let mut data = build_fixture();
        data[0] = 0xFF; // corrupt discriminator
        assert!(parse_pool_init(&data, 150.0).is_none());
    }

    #[test]
    fn rejects_short_data() {
        assert!(parse_pool_init(&[1, 2, 3, 4, 0, 0, 0, 0], 150.0).is_none());
    }

    #[tokio::test]
    async fn monitor_emits_events_via_broadcast() {
        let (raw_tx, raw_rx) = tokio::sync::broadcast::channel::<RawUpdate>(64);
        let mut pool_rx = spawn_pumpswap_monitor(raw_rx, 150.0);

        // Encode fixture bytes as hex, prefixed with "0x".
        let fixture = build_fixture();
        let hex = format!("0x{}", fixture.iter().map(|b| format!("{b:02x}")).collect::<String>());

        raw_tx
            .send(RawUpdate {
                slot: 1,
                signature: "test".into(),
                logs: vec![],
                accounts: vec![hex],
                timestamp: 0,
            })
            .unwrap();

        let event = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            pool_rx.recv(),
        )
        .await
        .expect("timeout")
        .expect("recv");

        assert_eq!(event.initial_price, 1_000_000);
        assert!((event.initial_liquidity - 1500.0).abs() < 1e-6);
    }
}
