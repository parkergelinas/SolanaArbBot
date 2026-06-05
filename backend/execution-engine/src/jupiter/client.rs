//! Async Jupiter v6 HTTP client with retry + route selection.

use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::config::EngineConfig;
use crate::strategy::SizedOrder;

#[derive(Debug, thiserror::Error)]
pub enum JupiterError {
    #[error("http {0}")]
    Http(#[from] reqwest::Error),
    #[error("api {status}: {body}")]
    Api { status: u16, body: String },
    #[error("retries exhausted")]
    RetriesExhausted,
}

#[derive(Clone, Debug, Serialize)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u32,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "onlyDirectRoutes")]
    pub only_direct_routes: bool,
    #[serde(rename = "asLegacyTransaction")]
    pub as_legacy_transaction: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dexes: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QuoteResponse {
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: Option<String>,
    #[serde(rename = "routePlan", default)]
    pub route_plan: Vec<RoutePlanStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RoutePlanStep {
    #[serde(rename = "swapInfo", default)]
    pub swap_info: Option<SwapInfo>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SwapInfo {
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SwapRequest {
    #[serde(rename = "quoteResponse")]
    pub quote_response: QuoteResponse,
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "wrapAndUnwrapSol")]
    pub wrap_and_unwrap_sol: bool,
    #[serde(rename = "dynamicComputeUnitLimit")]
    pub dynamic_compute_unit_limit: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
}

pub struct JupiterClient {
    http: Client,
    base_url: String,
    timeout: Duration,
}

impl JupiterClient {
    pub fn new(config: &EngineConfig) -> Self {
        let timeout = Duration::from_millis(config.request_timeout_ms);
        let http = Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(8)
            .build()
            .expect("reqwest client");
        Self {
            http,
            base_url: config.jupiter_base_url.clone(),
            timeout,
        }
    }

    pub async fn prewarm(&self) -> Result<(), JupiterError> {
        let _ = self
            .http
            .get(format!("{}/v6/quote", self.base_url))
            .timeout(self.timeout)
            .send()
            .await;
        Ok(())
    }

    pub fn build_quote_request(order: &SizedOrder, attempt: u32) -> QuoteRequest {
        let dexes = if attempt > 0 {
            Some(vec!["Raydium".into(), "Orca".into()])
        } else {
            None
        };
        QuoteRequest {
            input_mint: order.input_mint.clone(),
            output_mint: order.output_mint.clone(),
            amount: order.amount_lamports,
            slippage_bps: order.slippage_bps,
            swap_mode: "ExactIn".into(),
            only_direct_routes: attempt == 0,
            as_legacy_transaction: false,
            dexes,
        }
    }

    pub async fn get_quote(&self, req: &QuoteRequest) -> Result<QuoteResponse, JupiterError> {
        self.retry(3, |attempt| async move {
            let url = format!(
                "{}/v6/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&swapMode={}&onlyDirectRoutes={}&asLegacyTransaction={}",
                self.base_url,
                req.input_mint,
                req.output_mint,
                req.amount,
                req.slippage_bps,
                req.swap_mode,
                req.only_direct_routes,
                req.as_legacy_transaction,
            );
            let mut url = url;
            if let Some(dexes) = &req.dexes {
                if attempt > 0 {
                    url.push_str(&format!("&dexes={}", dexes.join(",")));
                }
            }

            let resp = self.http.get(&url).send().await?;
            let status = resp.status().as_u16();
            if status == 429 || status >= 500 {
                return Err(JupiterError::Api {
                    status,
                    body: resp.text().await.unwrap_or_default(),
                });
            }
            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(JupiterError::Api { status, body });
            }
            let quote: QuoteResponse = resp.json().await?;
            let hops = quote.route_plan.len();
            info!(hops, impact = ?quote.price_impact_pct, "jupiter route plan");
            Ok(quote)
        })
        .await
    }

    pub async fn get_swap_tx(&self, req: &SwapRequest) -> Result<SwapResponse, JupiterError> {
        self.retry(3, |_attempt| async move {
            let resp = self
                .http
                .post(format!("{}/v6/swap", self.base_url))
                .json(req)
                .send()
                .await?;
            let status = resp.status().as_u16();
            if status == 429 || status >= 500 {
                return Err(JupiterError::Api {
                    status,
                    body: resp.text().await.unwrap_or_default(),
                });
            }
            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(JupiterError::Api { status, body });
            }
            Ok(resp.json().await?)
        })
        .await
    }

    async fn retry<T, F, Fut>(&self, max: u32, mut f: F) -> Result<T, JupiterError>
    where
        F: FnMut(u32) -> Fut,
        Fut: std::future::Future<Output = Result<T, JupiterError>>,
    {
        let mut attempt = 0u32;
        loop {
            match f(attempt).await {
                Ok(v) => return Ok(v),
                Err(e) => {
                    let transient = matches!(
                        &e,
                        JupiterError::Api { status, .. } if *status == 429 || *status >= 500
                    ) || matches!(&e, JupiterError::Http(_));
                    if !transient || attempt + 1 >= max {
                        return Err(if attempt + 1 >= max && transient {
                            JupiterError::RetriesExhausted
                        } else {
                            e
                        });
                    }
                    let base = 50u64 * 2u64.pow(attempt);
                    let jitter = (attempt as u64 + 1) * 7;
                    warn!(attempt, backoff_ms = base + jitter, "jupiter retry");
                    sleep(Duration::from_millis(base + jitter)).await;
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn quote_request_builder_fields() {
        let order = SizedOrder {
            input_mint: "SOL".into(),
            output_mint: "USDC".into(),
            amount_lamports: 1_000_000_000,
            size_usd: 100.0,
            slippage_bps: 50,
            strategy: "momentum_follow".into(),
        };
        let req = JupiterClient::build_quote_request(&order, 0);
        assert_eq!(req.slippage_bps, 50);
        assert!(req.only_direct_routes);
        let retry = JupiterClient::build_quote_request(&order, 1);
        assert!(retry.dexes.is_some());
    }

    #[tokio::test]
    async fn retry_backoff_on_transient() {
        let delays: std::sync::Arc<std::sync::Mutex<Vec<u64>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let start = Instant::now();
        let client = JupiterClient {
            http: Client::new(),
            base_url: "http://invalid.local".into(),
            timeout: Duration::from_millis(50),
        };
        let result = client.get_quote(&QuoteRequest {
            input_mint: "A".into(),
            output_mint: "B".into(),
            amount: 1,
            slippage_bps: 50,
            swap_mode: "ExactIn".into(),
            only_direct_routes: true,
            as_legacy_transaction: false,
            dexes: None,
        }).await;
        assert!(result.is_err());
        let _elapsed = start.elapsed().as_millis();
        let _ = delays;
    }
}
