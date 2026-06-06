//! Unified Jupiter API client — `api.jup.ag` Price, Swap, and Tokens v2.

use serde::{Deserialize, Serialize};
use tracing::warn;

/// Primary Jupiter API gateway (replaces `price.jup.ag` and `quote-api.jup.ag`).
pub const JUPITER_API_BASE: &str = "https://api.jup.ag";

pub const JUPITER_PRICE_V3_URL: &str = "https://api.jup.ag/price/v3";
pub const JUPITER_SWAP_V1_BASE: &str = "https://api.jup.ag/swap/v1";
pub const JUPITER_TOKENS_V2_BASE: &str = "https://api.jup.ag/tokens/v2";

const LEGACY_PRICE_HOST: &str = "price.jup.ag";
const LEGACY_QUOTE_HOST: &str = "quote-api.jup.ag";

/// Attach optional `JUPITER_API_KEY` header.
pub fn jupiter_request(client: &reqwest::Client, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
    let mut req = client.request(method, url);
    if let Ok(key) = std::env::var("JUPITER_API_KEY") {
        let key = key.trim();
        if !key.is_empty() {
            req = req.header("x-api-key", key);
        }
    }
    req
}

pub fn jupiter_get(client: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    jupiter_request(client, reqwest::Method::GET, url)
}

pub fn normalize_jupiter_price_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.contains(LEGACY_PRICE_HOST) {
        warn!(
            configured = %trimmed,
            migrated = JUPITER_PRICE_V3_URL,
            "price.jup.ag deprecated — using api.jup.ag/price/v3"
        );
        return JUPITER_PRICE_V3_URL.to_owned();
    }
    trimmed.to_owned()
}

pub fn normalize_jupiter_swap_base(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.contains(LEGACY_QUOTE_HOST) {
        warn!(
            configured = %trimmed,
            migrated = JUPITER_SWAP_V1_BASE,
            "quote-api.jup.ag deprecated — using api.jup.ag/swap/v1"
        );
        return JUPITER_SWAP_V1_BASE.to_owned();
    }
    trimmed.to_owned()
}

/// Parse Jupiter Price API v2 (`data.{mint}.price`) or v3 (`{mint}.usdPrice`).
pub fn extract_jupiter_prices(body: &serde_json::Value) -> Vec<(String, f64)> {
    let mut out = Vec::new();

    if let Some(data) = body.get("data").and_then(|d| d.as_object()) {
        for (mint, entry) in data {
            if let Some(price) = entry.get("price").and_then(|p| p.as_f64()) {
                if price.is_finite() && price > 0.0 {
                    out.push((mint.clone(), price));
                }
            }
        }
        return out;
    }

    if let Some(obj) = body.as_object() {
        for (mint, entry) in obj {
            if let Some(price) = entry.get("usdPrice").and_then(|p| p.as_f64()) {
                if price.is_finite() && price > 0.0 {
                    out.push((mint.clone(), price));
                }
            }
        }
    }

    out
}

#[derive(Clone, Debug, Serialize)]
pub struct SwapQuoteRequest<'a> {
    pub input_mint: &'a str,
    pub output_mint: &'a str,
    pub amount: u64,
    pub slippage_bps: u32,
    pub restrict_intermediate_tokens: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SwapQuoteResponse {
    #[serde(default)]
    pub in_amount: String,
    #[serde(default)]
    pub out_amount: String,
    #[serde(default)]
    pub other_amount_threshold: String,
    #[serde(default)]
    pub price_impact_pct: f64,
    #[serde(default)]
    pub route_plan: Vec<serde_json::Value>,
}

impl SwapQuoteResponse {
    pub fn out_amount_u64(&self) -> Option<u64> {
        self.out_amount.parse().ok()
    }

    pub fn effective_price(&self, in_amount: u64, in_decimals: u8, out_decimals: u8) -> Option<f64> {
        let out = self.out_amount_u64()?;
        if in_amount == 0 || out == 0 {
            return None;
        }
        let in_ui = in_amount as f64 / 10f64.powi(in_decimals as i32);
        let out_ui = out as f64 / 10f64.powi(out_decimals as i32);
        if out_ui <= 0.0 {
            return None;
        }
        Some(in_ui / out_ui)
    }
}

pub fn build_quote_url(base: &str, req: &SwapQuoteRequest<'_>) -> String {
    let base = normalize_jupiter_swap_base(base);
    format!(
        "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&restrictIntermediateTokens={}",
        base,
        req.input_mint,
        req.output_mint,
        req.amount,
        req.slippage_bps,
        req.restrict_intermediate_tokens
    )
}

pub async fn fetch_swap_quote(
    client: &reqwest::Client,
    swap_base: &str,
    req: &SwapQuoteRequest<'_>,
) -> Result<SwapQuoteResponse, String> {
    let url = build_quote_url(swap_base, req);
    let resp = jupiter_get(client, &url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("jupiter quote HTTP {}", resp.status()));
    }
    resp.json::<SwapQuoteResponse>()
        .await
        .map_err(|e| e.to_string())
}

#[derive(Clone, Debug, Deserialize)]
pub struct JupiterTokenMeta {
    pub id: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub decimals: u8,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Fetch verified tokens from Tokens API v2 (`/tag?query=verified`).
pub async fn fetch_verified_tokens(
    client: &reqwest::Client,
    tokens_base: &str,
    limit: u32,
) -> Result<Vec<JupiterTokenMeta>, String> {
    let base = tokens_base.trim().trim_end_matches('/');
    let url = format!("{base}/tag?query=verified&limit={limit}");
    let resp = jupiter_get(client, &url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("jupiter tokens HTTP {}", resp.status()));
    }
    resp.json::<Vec<JupiterTokenMeta>>()
        .await
        .map_err(|e| e.to_string())
}

/// Quality gate — verified tag and optional strict list.
pub fn token_passes_quality(meta: &JupiterTokenMeta, require_strict: bool) -> bool {
    let has_verified = meta.tags.iter().any(|t| t.eq_ignore_ascii_case("verified"));
    if !has_verified {
        return false;
    }
    if require_strict {
        return meta.tags.iter().any(|t| t.eq_ignore_ascii_case("strict"));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_swap_host() {
        assert_eq!(
            normalize_jupiter_swap_base("https://quote-api.jup.ag/v6"),
            JUPITER_SWAP_V1_BASE
        );
    }

    #[test]
    fn parses_v3_prices() {
        let body = serde_json::json!({
            "So11111111111111111111111111111111111111112": { "usdPrice": 64.0 }
        });
        let p = extract_jupiter_prices(&body);
        assert_eq!(p.len(), 1);
    }
}
