//! Pool-creation event ingestion for the new-token sniper strategy.
//!
//! Prefers Yellowstone gRPC when `YELLOWSTONE_ENDPOINT` is set; otherwise falls
//! back to Helius `logsSubscribe` (same pattern as [`crate::helius`]).
//!
//! Detects:
//! - Raydium AMM V4 — `initialize2` in logs
//! - Pump.fun — `Instruction: Create`
//! - Raydium CLMM — `CreatePool`
//! - PumpSwap — `create_pool` / bonding-curve pool creation

use std::sync::Arc;
use std::time::Duration;

use config::DataSourcesConfig;
use crossbeam_channel::Sender;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info, warn};

pub const RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const PUMP_FUN: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const RAYDIUM_CLMM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
pub const PUMP_SWAP: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// DEX / launchpad that emitted a pool-creation event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolCreationSource {
    RaydiumAmmV4,
    PumpFun,
    RaydiumClmm,
    PumpSwap,
}

impl PoolCreationSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RaydiumAmmV4 => "raydium_amm_v4",
            Self::PumpFun => "pump_fun",
            Self::RaydiumClmm => "raydium_clmm",
            Self::PumpSwap => "pump_swap",
        }
    }
}

/// Pump.fun bonding-curve buy or sell detected from program logs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PumpTradeSide {
    Buy,
    Sell,
}

/// Parsed pump.fun trade from `buy` / `sell` instructions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PumpTradeEvent {
    pub side: PumpTradeSide,
    pub token_mint: String,
    pub wallet: String,
    pub signature: String,
    pub slot: u64,
    /// Best-effort SOL lamports from logs (0 when unknown).
    pub sol_lamports: u64,
}

/// Parsed new-pool event forwarded to the sniper executor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolCreationEvent {
    pub source: PoolCreationSource,
    pub token_mint: String,
    pub pool_address: String,
    pub creator_wallet: String,
    pub initial_liquidity_sol: f64,
    pub signature: String,
    pub slot: u64,
}

/// Spawn background ingestion (Geyser preferred, Helius fallback).
pub fn spawn_sniper_ingest(tx: Sender<PoolCreationEvent>, data_sources: Arc<DataSourcesConfig>) {
    if let Some(endpoint) = std::env::var("YELLOWSTONE_ENDPOINT")
        .ok()
        .filter(|s| !s.is_empty())
    {
        info!(endpoint = %endpoint, "sniper ingest: yellowstone gRPC preferred");
        crate::geyser::spawn_sniper_geyser(endpoint, tx);
        return;
    }

    let Some(ws_url) = data_sources.helius_ws_url() else {
        info!("sniper ingest skipped — no YELLOWSTONE_ENDPOINT or helius_api_key");
        return;
    };

    info!("sniper ingest: helius logsSubscribe fallback");
    tokio::spawn(async move {
        loop {
            if let Err(e) = subscribe_once(&ws_url, &tx).await {
                warn!(error = %e, "sniper helius stream disconnected, retry in 5s");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn subscribe_once(ws_url: &str, tx: &Sender<PoolCreationEvent>) -> anyhow::Result<()> {
    let (ws, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws.split();

    for program in [RAYDIUM_AMM_V4, PUMP_FUN, RAYDIUM_CLMM, PUMP_SWAP] {
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
            if let Some(event) = parse_pool_creation_notification(&text) {
                if tx.send(event).is_err() {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

/// Parse pump.fun buy/sell from transaction logs.
pub fn parse_pump_trade_from_logs(
    logs: &[String],
    signature: &str,
    slot: u64,
) -> Option<PumpTradeEvent> {
    let joined = logs.join("\n");
    if !joined.contains(PUMP_FUN) {
        return None;
    }

    let side = if joined.contains("Instruction: Buy") || joined.contains("Instruction: buy") {
        PumpTradeSide::Buy
    } else if joined.contains("Instruction: Sell") || joined.contains("Instruction: sell") {
        PumpTradeSide::Sell
    } else {
        return None;
    };

    let addresses = extract_base58_addresses(&joined);
    let token_mint = pick_token_mint(&addresses)?;
    let wallet = pick_creator(&addresses, &token_mint, "trade");
    let sol_lamports = extract_liquidity_sol(&joined)
        .map(|sol| (sol * 1_000_000_000.0) as u64)
        .unwrap_or(0);

    Some(PumpTradeEvent {
        side,
        token_mint,
        wallet,
        signature: signature.to_owned(),
        slot,
        sol_lamports,
    })
}

/// Parse a Helius `logsNotification` into a pump trade when matched.
pub fn parse_pump_trade_notification(text: &str) -> Option<PumpTradeEvent> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let result = v.pointer("/params/result/value")?;
    let signature = result.get("signature")?.as_str()?.to_owned();
    let slot = result
        .pointer("/context/slot")
        .and_then(|s| s.as_u64())
        .unwrap_or(0);
    let logs: Vec<String> = result
        .get("logs")?
        .as_array()?
        .iter()
        .filter_map(|l| l.as_str().map(str::to_owned))
        .collect();
    parse_pump_trade_from_logs(&logs, &signature, slot)
}

/// Spawn pump.fun trade ingestion (Helius logsSubscribe on pump program).
pub fn spawn_pump_trade_ingest(
    tx: Sender<PumpTradeEvent>,
    data_sources: Arc<DataSourcesConfig>,
) {
    let Some(ws_url) = data_sources.helius_ws_url() else {
        info!("pump trade ingest skipped — no helius_api_key");
        return;
    };

    info!("pump trade ingest: helius logsSubscribe");
    tokio::spawn(async move {
        loop {
            if let Err(e) = subscribe_pump_trades_once(&ws_url, &tx).await {
                warn!(error = %e, "pump trade stream disconnected, retry in 5s");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn subscribe_pump_trades_once(
    ws_url: &str,
    tx: &Sender<PumpTradeEvent>,
) -> anyhow::Result<()> {
    let (ws, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws.split();

    let sub = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "pump_trades",
        "method": "logsSubscribe",
        "params": [
            { "mentions": [PUMP_FUN] },
            { "commitment": "confirmed" }
        ]
    });
    write.send(Message::Text(sub.to_string())).await?;

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            if let Some(event) = parse_pump_trade_notification(&text) {
                if tx.send(event).is_err() {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

/// Parse a Helius `logsNotification` into a pool-creation event when matched.
pub fn parse_pool_creation_notification(text: &str) -> Option<PoolCreationEvent> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let result = v.pointer("/params/result/value")?;
    let signature = result.get("signature")?.as_str()?.to_owned();
    let slot = result
        .pointer("/context/slot")
        .and_then(|s| s.as_u64())
        .unwrap_or(0);
    let logs: Vec<String> = result
        .get("logs")?
        .as_array()?
        .iter()
        .filter_map(|l| l.as_str().map(str::to_owned))
        .collect();

    parse_pool_creation_from_logs(&logs, &signature, slot)
}

/// Core log-pattern matcher — unit-testable without WebSocket framing.
pub fn parse_pool_creation_from_logs(
    logs: &[String],
    signature: &str,
    slot: u64,
) -> Option<PoolCreationEvent> {
    let joined = logs.join("\n");

    let source = if joined.contains("initialize2") && joined.contains(RAYDIUM_AMM_V4) {
        PoolCreationSource::RaydiumAmmV4
    } else if joined.contains("Instruction: Create") && joined.contains(PUMP_FUN) {
        PoolCreationSource::PumpFun
    } else if joined.contains("CreatePool") && joined.contains(RAYDIUM_CLMM) {
        PoolCreationSource::RaydiumClmm
    } else if (joined.contains("create_pool") || joined.contains("CreatePool"))
        && joined.contains(PUMP_SWAP)
    {
        PoolCreationSource::PumpSwap
    } else {
        return None;
    };

    let addresses = extract_base58_addresses(&joined);
    let token_mint = pick_token_mint(&addresses)?;
    let pool_address = pick_pool_address(&addresses, &token_mint, source);
    let creator_wallet = pick_creator(&addresses, &token_mint, &pool_address);
    let initial_liquidity_sol = extract_liquidity_sol(&joined).unwrap_or(0.0);

    debug!(
        source = source.as_str(),
        mint = %token_mint,
        pool = %pool_address,
        liquidity_sol = initial_liquidity_sol,
        signature,
        "pool creation detected"
    );

    Some(PoolCreationEvent {
        source,
        token_mint,
        pool_address,
        creator_wallet,
        initial_liquidity_sol,
        signature: signature.to_owned(),
        slot,
    })
}

fn extract_base58_addresses(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        let cleaned: String = word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_owned();
        if cleaned.len() >= 32 && cleaned.len() <= 44 && is_base58(&cleaned) {
            if !out.contains(&cleaned) {
                out.push(cleaned);
            }
        }
    }
    out
}

fn is_base58(s: &str) -> bool {
    const ALPHABET: &str =
        "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    s.chars().all(|c| ALPHABET.contains(c))
}

fn pick_token_mint(addresses: &[String]) -> Option<String> {
    addresses
        .iter()
        .find(|a| *a != SOL_MINT && !is_program_id(a))
        .cloned()
}

fn pick_pool_address(addresses: &[String], mint: &str, source: PoolCreationSource) -> String {
    addresses
        .iter()
        .find(|a| *a != mint && *a != SOL_MINT && !is_program_id(a))
        .cloned()
        .unwrap_or_else(|| format!("pool_{source:?}_{mint}"))
}

fn pick_creator(addresses: &[String], mint: &str, pool: &str) -> String {
    addresses
        .iter()
        .find(|a| *a != mint && *a != pool && *a != SOL_MINT && !is_program_id(a))
        .cloned()
        .unwrap_or_else(|| "unknown_creator".to_owned())
}

fn is_program_id(addr: &str) -> bool {
    matches!(
        addr,
        RAYDIUM_AMM_V4 | PUMP_FUN | RAYDIUM_CLMM | PUMP_SWAP | SOL_MINT
    )
}

fn extract_liquidity_sol(logs: &str) -> Option<f64> {
    for line in logs.lines() {
        let lower = line.to_lowercase();
        if lower.contains("lamports") || lower.contains("sol") {
            for word in line.split_whitespace() {
                if let Ok(v) = word.trim_matches(|c: char| !c.is_ascii_digit()).parse::<f64>() {
                    if lower.contains("lamports") {
                        return Some(v / 1_000_000_000.0);
                    }
                    // Values >= 1e9 on a "sol"-tagged line are almost certainly lamports.
                    if v >= 1_000_000_000.0 {
                        return Some(v / 1_000_000_000.0);
                    }
                    if v > 0.0 && v < 1_000_000.0 {
                        return Some(v);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pump_logs() -> Vec<String> {
        vec![
            format!("Program {PUMP_FUN} invoke [1]"),
            "Program log: Instruction: Create".to_owned(),
            "Program log: mint: 7GCihgDB8fe6KNjn2MYtkzZcRjZy3q9jtK68W89Qjcb2".to_owned(),
            "Program log: sol_amount: 8500000000".to_owned(),
        ]
    }

    #[test]
    fn detects_pump_fun_create() {
        let logs = pump_logs();
        let ev = parse_pool_creation_from_logs(&logs, "sig1", 100).expect("event");
        assert_eq!(ev.source, PoolCreationSource::PumpFun);
        assert_eq!(
            ev.token_mint,
            "7GCihgDB8fe6KNjn2MYtkzZcRjZy3q9jtK68W89Qjcb2"
        );
        assert!(ev.initial_liquidity_sol > 0.0);
    }

    #[test]
    fn detects_raydium_initialize2() {
        let logs = vec![
            format!("Program {RAYDIUM_AMM_V4} invoke [1]"),
            "Program log: initialize2".to_owned(),
            "Program log: mint: TokenMintRaydium111111111111111111111111".to_owned(),
        ];
        let ev = parse_pool_creation_from_logs(&logs, "sig2", 200).expect("event");
        assert_eq!(ev.source, PoolCreationSource::RaydiumAmmV4);
    }

    #[test]
    fn ignores_swap_logs() {
        let logs = vec![
            format!("Program {RAYDIUM_AMM_V4} invoke [1]"),
            "Program log: swap".to_owned(),
        ];
        assert!(parse_pool_creation_from_logs(&logs, "sig3", 0).is_none());
    }

    #[test]
    fn detects_pump_buy() {
        let logs = vec![
            format!("Program {PUMP_FUN} invoke [1]"),
            "Program log: Instruction: Buy".to_owned(),
            "Program log: mint: 7GCihgDB8fe6KNjn2MYtkzZcRjZy3q9jtK68W89Qjcb2".to_owned(),
            "Program log: sol_amount: 500000000".to_owned(),
        ];
        let ev = parse_pump_trade_from_logs(&logs, "sig4", 101).expect("trade");
        assert_eq!(ev.side, PumpTradeSide::Buy);
        assert_eq!(
            ev.token_mint,
            "7GCihgDB8fe6KNjn2MYtkzZcRjZy3q9jtK68W89Qjcb2"
        );
    }
}
