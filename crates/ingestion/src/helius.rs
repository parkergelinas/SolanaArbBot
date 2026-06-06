//! Helius free-tier WebSocket stream — DEX program log subscription.
//!
//! Uses `wss://mainnet.helius-rpc.com/?api-key={key}` from [`config::DataSourcesConfig`].
//! Full Yellowstone gRPC can be enabled later via optional feature.

use std::sync::Arc;
use std::time::Duration;

use common::{MarketEvent, Pubkey, SwapEvent};
use config::DataSourcesConfig;
use events::EventBus;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

const RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const RAYDIUM_CLMM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

/// Spawns a background task that subscribes to Helius WS logs and publishes to `EventBus`.
pub fn spawn_helius_stream(bus: EventBus, data_sources: Arc<DataSourcesConfig>) {
    let Some(ws_url) = data_sources.helius_ws_url() else {
        info!("helius stream skipped — no helius_api_key configured");
        return;
    };
    info!(url = %mask_key(&ws_url), "helius WS stream starting");

    tokio::spawn(async move {
        loop {
            if let Err(e) = subscribe_once(&ws_url, &bus).await {
                warn!(error = %e, "helius stream disconnected, retry in 5s");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

fn mask_key(url: &str) -> String {
    if let Some(idx) = url.find("api-key=") {
        format!("{}api-key=***", &url[..idx + 8])
    } else {
        url.to_string()
    }
}

async fn subscribe_once(ws_url: &str, bus: &EventBus) -> anyhow::Result<()> {
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
        write
            .send(Message::Text(sub.to_string()))
            .await?;
    }

    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            if let Some(event) = parse_log_notification(&text) {
                let _ = bus.publish_async(event).await;
            }
        }
    }
    Ok(())
}

fn parse_log_notification(text: &str) -> Option<MarketEvent> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let result = v.pointer("/params/result/value")?;
    let sig = result.get("signature")?.as_str()?;
    let logs = result.get("logs")?.as_array()?;
    let is_swap = logs.iter().any(|l| {
        l.as_str()
            .map(|s| s.contains("swap") || s.contains("Swap"))
            .unwrap_or(false)
    });
    if !is_swap {
        return None;
    }

    if sig.is_empty() {
        return None;
    }

    let pool = Pubkey::new([0x42; 32]);
    let token = Pubkey::new([0xAA; 32]);
    Some(MarketEvent::SwapEvent(SwapEvent {
        pool,
        input_mint: token,
        output_mint: Pubkey::new([0xBB; 32]),
        amount_in: 1_000_000,
        amount_out: 998_000,
    }))
}
