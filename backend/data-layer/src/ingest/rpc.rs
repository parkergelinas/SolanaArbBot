//! Solana JSON-RPC / WebSocket log subscription — live chain swap detection.

use std::time::Duration;

use crossbeam_channel::Sender;
use tracing::{info, warn};

use super::adapter::IngestionAdapter;
use crate::types::RawUpdate;

/// Raydium AMM v4 — high-volume DEX program for log subscription.
const RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const JUPITER_V6: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

pub struct RpcAdapter {
    pub ws_url: String,
}

impl RpcAdapter {
    pub fn from_env() -> Option<Self> {
        let ws = std::env::var("SOLANA_ARB_DATA_SOURCES__HELIUS_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .map(|key| format!("wss://mainnet.helius-rpc.com/?api-key={key}"))
            .or_else(|| std::env::var("SOLANA_RPC_WS").ok().filter(|s| !s.is_empty()))
            .or_else(|| {
                std::env::var("SOLANA_ARB_WEBSOCKET__ENDPOINTS")
                    .ok()
                    .and_then(|s| s.split(',').next().map(str::trim).map(String::from))
            })
            .or_else(|| {
                std::env::var("SOLANA_ARB_RPC__ENDPOINTS")
                    .ok()
                    .and_then(|s| {
                        s.split(',')
                            .next()
                            .map(str::trim)
                            .map(http_to_ws)
                    })
            })?;

        Some(Self { ws_url: ws })
    }
}

fn http_to_ws(http: &str) -> String {
    http.replace("https://", "wss://")
        .replace("http://", "ws://")
}

impl IngestionAdapter for RpcAdapter {
    fn name(&self) -> &'static str {
        "rpc"
    }

    fn spawn(self: Box<Self>, tx: Sender<RawUpdate>) -> anyhow::Result<()> {
        let ws_url = self.ws_url.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => {
                    warn!(error = %e, "rpc adapter: failed to start tokio runtime");
                    return;
                }
            };
            rt.block_on(run_logs_subscriber(ws_url, tx));
        });
        Ok(())
    }
}

async fn run_logs_subscriber(ws_url: String, tx: Sender<RawUpdate>) {
    loop {
        if let Err(e) = subscribe_once(&ws_url, &tx).await {
            warn!(error = %e, url = %ws_url, "rpc logsSubscribe disconnected, retry in 5s");
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn subscribe_once(ws_url: &str, tx: &Sender<RawUpdate>) -> anyhow::Result<()> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    info!(url = %ws_url, "rpc adapter: connecting logsSubscribe");
    let (ws, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws.split();

    for (id, program) in [(1, RAYDIUM_AMM_V4), (2, ORCA_WHIRLPOOL), (3, JUPITER_V6)] {
        let sub = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "logsSubscribe",
            "params": [
                { "mentions": [program] },
                { "commitment": "confirmed" }
            ]
        });
        write
            .send(Message::Text(sub.to_string()))
            .await?;
    }

    info!("rpc adapter: subscribed to DEX program logs (raydium, orca, jupiter)");

    while let Some(msg) = read.next().await {
        let msg = msg?;
        let Message::Text(text) = msg else { continue };

        if let Some(update) = parse_logs_notification(&text) {
            if tx.send(update).is_err() {
                break;
            }
        }
    }
    Ok(())
}

fn parse_logs_notification(text: &str) -> Option<RawUpdate> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if v.get("method")?.as_str()? != "logsNotification" {
        return None;
    }

    let result = v.pointer("/params/result/value")?;
    let signature = result.get("signature")?.as_str()?.to_string();
    let logs: Vec<String> = result
        .pointer("/logs")?
        .as_array()?
        .iter()
        .filter_map(|l| l.as_str().map(String::from))
        .collect();

    let slot = v
        .pointer("/params/result/context/slot")
        .and_then(|s| s.as_u64())
        .unwrap_or(0);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let accounts = extract_accounts_from_logs(&logs);

    Some(RawUpdate {
        slot,
        signature,
        logs,
        accounts,
        timestamp,
    })
}

fn extract_accounts_from_logs(logs: &[String]) -> Vec<String> {
    let mut accounts = Vec::new();
    for log in logs {
        let lower = log.to_lowercase();
        if lower.contains("raydium") {
            accounts.push(RAYDIUM_AMM_V4.into());
        }
        if lower.contains("orca") || lower.contains("whirlpool") {
            accounts.push(ORCA_WHIRLPOOL.into());
        }
        if lower.contains("jupiter") {
            accounts.push(JUPITER_V6.into());
        }
    }
    accounts.dedup();
    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_logs_notification() {
        let json = r#"{
            "jsonrpc":"2.0",
            "method":"logsNotification",
            "params":{
                "result":{
                    "context":{"slot":123},
                    "value":{
                        "signature":"5abc",
                        "logs":["Program log: raydium swap","Program log: Instruction: Swap"]
                    }
                }
            }
        }"#;
        let u = parse_logs_notification(json).expect("parsed");
        assert_eq!(u.signature, "5abc");
        assert_eq!(u.slot, 123);
        assert!(!u.logs.is_empty());
    }

    #[test]
    fn http_to_ws_converts() {
        assert_eq!(
            http_to_ws("https://api.mainnet-beta.solana.com"),
            "wss://api.mainnet-beta.solana.com"
        );
    }
}
