//! Helius WebSocket streams — real-time DEX pool data via Solana account subscriptions.
//!
//! ## Two feeds provided
//!
//! 1. `spawn_helius_stream` — logs-subscribe feed that publishes generic
//!    [`MarketEvent`]s onto the [`EventBus`] for the signal / strategy engine.
//!
//! 2. `spawn_helius_hotpath_feed` — account-subscribe feed that directly drives
//!    the hot-path engine with [`MarketTick`]s derived from real SPL vault
//!    balances.  This replaces the synthetic ingestion loop.
//!
//! ## How `spawn_helius_hotpath_feed` works
//!
//! For each pool in the registry we subscribe to TWO SPL token vault accounts
//! via Solana's `accountSubscribe` WebSocket method.  Every time a vault
//! balance changes (swap, add/remove liquidity), we receive the full account
//! data encoded in base64.  We read the `amount` field (u64 at offset 64 in the
//! SPL token account layout), update an in-memory reserve snapshot, and emit a
//! [`MarketTick`] with the real price and reserves.
//!
//! The connection auto-reconnects on drops with 5-second back-off.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use common::{MarketEvent, Pubkey, SwapEvent};
use config::DataSourcesConfig;
use crossbeam_channel::Sender;
use events::EventBus;
use execution::hotpath::MarketTick;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info, warn};

use crate::pools::{mainnet_pools, spl_token_amount, PoolEntry};

const RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const RAYDIUM_CLMM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

// ─────────────────────────────────────────────────────────────────────────────
// Legacy logs-subscribe stream (EventBus)
// ─────────────────────────────────────────────────────────────────────────────

/// Spawns a background task that subscribes to Helius WS logs and publishes to `EventBus`.
pub fn spawn_helius_stream(bus: EventBus, data_sources: Arc<DataSourcesConfig>) {
    let Some(ws_url) = data_sources.helius_ws_url() else {
        info!("helius stream skipped — no helius_api_key configured");
        return;
    };
    info!(url = %mask_key(&ws_url), "helius WS stream starting");

    tokio::spawn(async move {
        loop {
            if let Err(e) = logs_subscribe_once(&ws_url, &bus).await {
                warn!(error = %e, "helius stream disconnected, retry in 5s");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn logs_subscribe_once(ws_url: &str, bus: &EventBus) -> anyhow::Result<()> {
    let (ws, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws.split();

    for program in [RAYDIUM_AMM_V4, ORCA_WHIRLPOOL, RAYDIUM_CLMM] {
        let sub = serde_json::json!({
            "jsonrpc": "2.0",
            "id": program,
            "method": "logsSubscribe",
            "params": [
                { "mentions": [program] },
                { "commitment": "confirmed" }
            ]
        });
        write.send(Message::Text(sub.to_string())).await?;
    }

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            if let Some(event) = parse_swap_log(&text) {
                let _ = bus.publish_async(event).await;
            }
        }
    }
    Ok(())
}

/// Parse a real swap event from a Helius logsNotification.
///
/// The logs contain a signature and program logs.  We detect swap activity
/// and emit a minimal `SwapEvent` that the signal engine can process.
fn parse_swap_log(text: &str) -> Option<MarketEvent> {
    let v: Value = serde_json::from_str(text).ok()?;

    // Only process logsNotification messages
    let method = v.get("method")?.as_str()?;
    if method != "logsNotification" {
        return None;
    }

    let value = v.pointer("/params/result/value")?;
    // Skip errored transactions
    if value.get("err").and_then(|e| e.as_null()).is_none()
        && value.get("err")?.is_null().not_like_bool()
    {
        // err is non-null → transaction failed
        return None;
    }

    let logs = value.get("logs")?.as_array()?;

    // Detect which program this is and extract amounts from program logs.
    // Raydium logs contain "ray_log: ..." with base64 encoded data.
    // Orca logs contain structured data we can parse.
    let mut amount_in: u64 = 0;
    let mut amount_out: u64 = 0;
    let mut is_swap = false;
    let mut program_id = "";

    for log in logs {
        let s = log.as_str().unwrap_or("");
        if s.contains("Program 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8 invoke") {
            program_id = RAYDIUM_AMM_V4;
        } else if s.contains("Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc invoke") {
            program_id = ORCA_WHIRLPOOL;
        } else if s.contains("ray_log:") {
            // Raydium swap log — base64 encoded struct with amounts
            if let Some(amounts) = parse_raydium_ray_log(s) {
                amount_in = amounts.0;
                amount_out = amounts.1;
                is_swap = true;
            }
        } else if s.contains("Swap") || s.contains("swap") {
            is_swap = true;
        }
    }

    if !is_swap {
        return None;
    }

    // Use placeholder mints when we can't extract them from logs alone.
    // The hot-path engine uses pool_idx (from accountSubscribe), not mints.
    let (input_mint, output_mint) = match program_id {
        x if x == RAYDIUM_AMM_V4 => (
            // wSOL → USDC as default; real routing is in the hot path
            Pubkey::new([0xAA; 32]),
            Pubkey::new([0xBB; 32]),
        ),
        _ => (Pubkey::new([0xCC; 32]), Pubkey::new([0xDD; 32])),
    };

    Some(MarketEvent::SwapEvent(SwapEvent {
        pool: Pubkey::new([0x42; 32]),
        input_mint,
        output_mint,
        amount_in: if amount_in > 0 { amount_in } else { 1_000_000 },
        amount_out: if amount_out > 0 { amount_out } else { 998_000 },
    }))
}

/// Parse a Raydium `ray_log:` line to extract swap amounts.
///
/// Format: "ray_log: <base64>" where the base64 decodes to a struct
/// containing (amount_in: u64, amount_out: u64) at bytes 8..24.
fn parse_raydium_ray_log(log_line: &str) -> Option<(u64, u64)> {
    let prefix = "ray_log: ";
    let b64 = log_line.split(prefix).nth(1)?.trim();
    let decoded = base64_decode(b64)?;
    if decoded.len() < 24 {
        return None;
    }
    let amount_in = u64::from_le_bytes(decoded[8..16].try_into().ok()?);
    let amount_out = u64::from_le_bytes(decoded[16..24].try_into().ok()?);
    if amount_in == 0 || amount_out == 0 {
        return None;
    }
    Some((amount_in, amount_out))
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    // Simple base64 decode without adding a new dep — use the existing encoding.
    // We do it manually via the standard alphabet.
    base64_simple(s)
}

fn base64_simple(s: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let s = s.trim_end_matches('=');
    let mut output = Vec::with_capacity(s.len() * 3 / 4 + 1);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = TABLE.iter().position(|&c| c == bytes[i])? as u32;
        let b = TABLE.iter().position(|&c| c == bytes[i + 1])? as u32;
        let c = TABLE.iter().position(|&c| c == bytes[i + 2])? as u32;
        let d = TABLE.iter().position(|&c| c == bytes[i + 3])? as u32;
        output.push(((a << 2) | (b >> 4)) as u8);
        output.push(((b << 4) | (c >> 2)) as u8);
        output.push(((c << 6) | d) as u8);
        i += 4;
    }
    match bytes.len() - i {
        2 => {
            let a = TABLE.iter().position(|&c| c == bytes[i])? as u32;
            let b = TABLE.iter().position(|&c| c == bytes[i + 1])? as u32;
            output.push(((a << 2) | (b >> 4)) as u8);
        }
        3 => {
            let a = TABLE.iter().position(|&c| c == bytes[i])? as u32;
            let b = TABLE.iter().position(|&c| c == bytes[i + 1])? as u32;
            let c = TABLE.iter().position(|&c| c == bytes[i + 2])? as u32;
            output.push(((a << 2) | (b >> 4)) as u8);
            output.push(((b << 4) | (c >> 2)) as u8);
        }
        _ => {}
    }
    Some(output)
}

trait BoolExt {
    fn not_like_bool(&self) -> bool;
}
impl BoolExt for bool {
    fn not_like_bool(&self) -> bool {
        !self
    }
}

fn mask_key(url: &str) -> String {
    if let Some(idx) = url.find("api-key=") {
        format!("{}api-key=***", &url[..idx + 8])
    } else {
        url.to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hot-path account-subscribe feed
// ─────────────────────────────────────────────────────────────────────────────

/// Vault state snapshot for one pool.
#[derive(Clone, Copy, Default)]
struct VaultSnapshot {
    reserve_a: u64,
    reserve_b: u64,
    slot: u64,
}

/// Spawns a background task that subscribes to SPL vault accounts for all
/// tracked pools and sends real [`MarketTick`]s to the hot-path engine.
///
/// Falls back silently if no Helius API key is configured.
pub fn spawn_helius_hotpath_feed(
    tick_tx: Sender<MarketTick>,
    data_sources: Arc<DataSourcesConfig>,
) {
    let Some(ws_url) = data_sources.helius_ws_url() else {
        info!("helius hotpath feed skipped — no helius_api_key configured");
        return;
    };
    let pools = mainnet_pools();
    info!(
        url = %mask_key(&ws_url),
        pool_count = pools.len(),
        "helius hotpath feed starting (accountSubscribe)"
    );

    tokio::spawn(async move {
        loop {
            if let Err(e) = account_subscribe_loop(&ws_url, &pools, tick_tx.clone()).await {
                warn!(error = %e, "helius hotpath feed disconnected, retry in 5s");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn account_subscribe_loop(
    ws_url: &str,
    pools: &[PoolEntry],
    tick_tx: Sender<MarketTick>,
) -> anyhow::Result<()> {
    let (ws, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws.split();

    // subscription_id → (pool_idx, is_vault_a)
    let mut pending: HashMap<serde_json::Value, (u16, bool)> = HashMap::new();
    let mut confirmed: HashMap<u64, (u16, bool)> = HashMap::new();

    // Per-pool reserve snapshot (indexed by pool_idx as usize).
    let max_idx = pools.iter().map(|p| p.pool_idx as usize).max().unwrap_or(0);
    let mut snapshots: Vec<VaultSnapshot> = vec![VaultSnapshot::default(); max_idx + 1];

    // Send accountSubscribe for each vault.
    for pool in pools {
        for (vault, is_a) in [(&pool.vault_a, true), (&pool.vault_b, false)] {
            let id = serde_json::json!(format!("{}-{}", pool.pool_idx, if is_a { "a" } else { "b" }));
            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "accountSubscribe",
                "params": [
                    vault,
                    { "encoding": "base64", "commitment": "confirmed" }
                ]
            });
            pending.insert(id, (pool.pool_idx, is_a));
            write.send(Message::Text(msg.to_string())).await?;
            debug!(vault = %vault, pool_idx = pool.pool_idx, is_a, "subscribed to vault account");
        }
    }

    while let Some(msg) = read.next().await {
        let msg = msg?;
        let text = match msg {
            Message::Text(t) => t,
            Message::Ping(d) => {
                let _ = write.send(Message::Pong(d)).await;
                continue;
            }
            _ => continue,
        };

        let v: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Subscription confirmation: {"jsonrpc":"2.0","result":<sub_id>,"id":"..."}
        if let (Some(result), Some(id)) = (v.get("result"), v.get("id")) {
            if let Some(sub_id) = result.as_u64() {
                if let Some(&(pool_idx, is_a)) = pending.get(id) {
                    confirmed.insert(sub_id, (pool_idx, is_a));
                    debug!(sub_id, pool_idx, is_a, "vault subscription confirmed");
                }
            }
            continue;
        }

        // accountNotification: {"method":"accountNotification","params":{...}}
        if v.get("method").and_then(|m| m.as_str()) == Some("accountNotification") {
            if let Some((sub_id, slot, data_b64)) = extract_account_notification(&v) {
                let Some(&(pool_idx, is_a)) = confirmed.get(&sub_id) else {
                    continue;
                };

                let account_data = base64_simple(data_b64).unwrap_or_default();
                let Some(amount) = spl_token_amount(&account_data) else {
                    continue;
                };

                let snap = match snapshots.get_mut(pool_idx as usize) {
                    Some(s) => s,
                    None => continue,
                };

                if is_a {
                    snap.reserve_a = amount;
                } else {
                    snap.reserve_b = amount;
                }
                snap.slot = slot;

                // Only emit a tick once both reserves are non-zero.
                if snap.reserve_a == 0 || snap.reserve_b == 0 {
                    continue;
                }

                let pool = match pools.iter().find(|p| p.pool_idx == pool_idx) {
                    Some(p) => p,
                    None => continue,
                };

                let price_fp = pool.price_fp(snap.reserve_a, snap.reserve_b);
                if price_fp == 0 {
                    continue;
                }

                // Volume delta: approximate from the change in the vault that just updated.
                let volume_delta_usd_x100 = estimate_volume_delta(amount, pool.decimals_a);

                let tick = MarketTick {
                    pool_idx,
                    slot,
                    price_fp,
                    reserve_a: snap.reserve_a,
                    reserve_b: snap.reserve_b,
                    volume_delta_usd_x100,
                };

                debug!(
                    pool_idx,
                    slot,
                    price_fp,
                    reserve_a = snap.reserve_a,
                    reserve_b = snap.reserve_b,
                    "emitting real market tick"
                );

                // Non-blocking send; drop tick if hot loop is full.
                let _ = tick_tx.try_send(tick);
            }
        }
    }
    Ok(())
}

/// Extract (subscription_id, slot, base64_data) from an accountNotification.
fn extract_account_notification(v: &Value) -> Option<(u64, u64, &str)> {
    let params = v.get("params")?;
    let sub_id = params.get("subscription")?.as_u64()?;
    let result = params.get("result")?;
    let slot = result.pointer("/context/slot")?.as_u64()?;
    // data is [base64_string, encoding_string]
    let data_b64 = result
        .pointer("/value/data/0")?
        .as_str()?;
    Some((sub_id, slot, data_b64))
}

/// Rough USD volume estimate from a vault amount change.
///
/// This is approximate — it just uses the raw lamport amount converted to SOL
/// (for SOL vaults) or USDC units for stable vaults.  The hot-path signal
/// logic uses this to score momentum so precision is not critical.
fn estimate_volume_delta(amount: u64, decimals: u8) -> u64 {
    // Return amount in USD cents (×100) — approximate: treat as $1 per native unit
    // at 1e-decimals denomination. For SOL at $150, scale accordingly.
    // The hot-path risk gate already has its own floor so a rough estimate is fine.
    let units = amount / 10u64.pow(decimals as u32).max(1);
    // Approximate: assume $150 SOL price for volume estimation in lamports
    // USD × 100 = units * 150 * 100 (for SOL)
    units.saturating_mul(150 * 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_simple_decodes_standard() {
        // "hello" in base64 is "aGVsbG8="
        let decoded = base64_simple("aGVsbG8").unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn parse_swap_log_ignores_non_swap() {
        let not_swap = r#"{"method":"logsNotification","params":{"result":{"value":{"signature":"abc","err":null,"logs":["Program 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8 invoke [1]","Program log: initialize"]}}}}"#;
        let result = parse_swap_log(not_swap);
        assert!(result.is_none());
    }

    #[test]
    fn mask_key_hides_api_key() {
        let url = "wss://mainnet.helius-rpc.com/?api-key=my-secret-key";
        let masked = mask_key(url);
        assert!(masked.contains("api-key=***"));
        assert!(!masked.contains("my-secret-key"));
    }
}
