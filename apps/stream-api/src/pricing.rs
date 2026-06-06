//! Jupiter Price API poller â€” emits `token_price` WS messages for watchlist mints.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::Sender;
use pricing::{extract_jupiter_prices, normalize_jupiter_price_url, JUPITER_PRICE_V3_URL};
use tracing::{debug, warn};

use crate::contracts::{TokenPrice, WSMessage, SCHEMA_VERSION};

/// Default terminal watchlist mints (aligned with `apps/dashboard/lib/terminal/tokens.ts`).
const DEFAULT_MINTS: &[&str] = &[
    "So11111111111111111111111111111111111111112",
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
    "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
    "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
];

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn watchlist_mints() -> Vec<String> {
    std::env::var("STREAM_JUPITER_MINTS")
        .ok()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|m| !m.is_empty())
                .map(String::from)
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_MINTS.iter().map(|s| (*s).to_string()).collect())
}

fn jupiter_price_url() -> String {
    let configured = std::env::var("STREAM_JUPITER_PRICE_URL")
        .unwrap_or_else(|_| JUPITER_PRICE_V3_URL.to_string());
    normalize_jupiter_price_url(&configured)
}

fn jupiter_get(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    let mut req = client.get(url);
    if let Ok(key) = std::env::var("JUPITER_API_KEY") {
        let key = key.trim();
        if !key.is_empty() {
            req = req.header("x-api-key", key);
        }
    }
    req
}

/// Poll Jupiter and emit `token_price` WS messages (500 ms default cadence).
pub fn spawn_jupiter_price_poller(ws_tx: Sender<WSMessage>) {
    let poll_ms: u64 = std::env::var("STREAM_JUPITER_POLL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);

    let disabled = std::env::var("STREAM_JUPITER_POLL")
        .map(|v| v == "0" || v.eq_ignore_ascii_case("false"))
        .unwrap_or(false);

    if disabled {
        debug!("jupiter price poller disabled (STREAM_JUPITER_POLL=0)");
        return;
    }

    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let mints = watchlist_mints();
        let base_url = jupiter_price_url();
        let mut cache: HashMap<String, f64> = HashMap::new();
        let mut fail_streak: u32 = 0;

        debug!(poll_ms, mint_count = mints.len(), base_url = %base_url, "jupiter price poller online");

        loop {
            if !mints.is_empty() {
                let url = format!("{}?ids={}", base_url, mints.join(","));

                match jupiter_get(&client, &url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        fail_streak = 0;
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            publish_jupiter_prices(&ws_tx, &mut cache, &body);
                        }
                    }
                    Ok(resp) if resp.status().as_u16() == 429 => {
                        fail_streak += 1;
                        warn!("jupiter price rate limited â€” backing off");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                    Ok(resp) => {
                        fail_streak += 1;
                        warn!(status = %resp.status(), url = %base_url, "jupiter price poll failed");
                    }
                    Err(e) => {
                        fail_streak += 1;
                        if fail_streak == 1 || fail_streak % 20 == 0 {
                            warn!(
                                error = %e,
                                url = %base_url,
                                "jupiter price request error (check api.jup.ag/price/v3; price.jup.ag is retired)"
                            );
                        }
                    }
                }
            }

            let backoff_ms = if fail_streak > 3 {
                poll_ms.saturating_mul(fail_streak.min(10) as u64)
            } else {
                poll_ms
            };
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        }
    });
}

fn publish_jupiter_prices(
    ws_tx: &Sender<WSMessage>,
    cache: &mut HashMap<String, f64>,
    body: &serde_json::Value,
) {
    let ts = now_ms();
    for (mint, price) in extract_jupiter_prices(body) {
        let prev = cache.get(&mint).copied().unwrap_or(price);
        let delta_pct = ((price - prev) / prev.max(1e-12)).abs() * 100.0;
        cache.insert(mint.clone(), price);

        // Skip tiny moves after warm-up to reduce WS noise.
        if delta_pct < 0.02 && cache.len() > 1 {
            continue;
        }

        let msg = WSMessage::TokenPrice(TokenPrice {
            v: SCHEMA_VERSION,
            mint: mint.clone(),
            price_usd: price,
            slot: 0,
            timestamp_ms: ts,
        });
        let _ = ws_tx.send(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jupiter_v3_body() {
        let body = serde_json::json!({
            "So11111111111111111111111111111111111111112": { "usdPrice": 63.42 }
        });
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut cache = HashMap::new();
        publish_jupiter_prices(&tx, &mut cache, &body);
        let msg = rx.recv().expect("price emitted");
        match msg {
            WSMessage::TokenPrice(p) => assert!((p.price_usd - 63.42).abs() < f64::EPSILON),
            _ => panic!("expected token_price"),
        }
    }

    #[test]
    fn parses_jupiter_v2_body() {
        let body = serde_json::json!({
            "data": {
                "So11111111111111111111111111111111111111112": { "price": 145.5 }
            }
        });
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut cache = HashMap::new();
        publish_jupiter_prices(&tx, &mut cache, &body);
        let msg = rx.recv().expect("price emitted");
        match msg {
            WSMessage::TokenPrice(p) => assert_eq!(p.price_usd, 145.5),
            _ => panic!("expected token_price"),
        }
    }
}

