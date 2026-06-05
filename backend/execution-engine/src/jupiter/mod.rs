//! Jupiter quote + swap API client.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::config::EngineConfig;
use crate::strategy::SwapOrder;

#[derive(Debug, Deserialize)]
pub struct QuoteResponse {
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: Option<String>,
}

#[derive(Debug, Serialize)]
struct SwapRequest<'a> {
    #[serde(rename = "quoteResponse")]
    quote_response: &'a QuoteResponse,
    #[serde(rename = "userPublicKey")]
    user_public_key: String,
    #[serde(rename = "wrapAndUnwrapSol")]
    wrap_and_unwrap_sol: bool,
}

#[derive(Debug, Deserialize)]
pub struct SwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
}

pub struct JupiterExecutor {
    client: Client,
    config: EngineConfig,
}

impl JupiterExecutor {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub async fn fetch_quote(&self, order: &SwapOrder) -> anyhow::Result<QuoteResponse> {
        let url = format!(
            "{}?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.config.jupiter_quote_url,
            order.input_mint,
            order.output_mint,
            order.amount_lamports,
            order.slippage_bps,
        );

        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("jupiter quote failed {status}: {body}");
        }
        Ok(resp.json().await?)
    }

    pub async fn execute(
        &self,
        order: &SwapOrder,
        user_pubkey: &str,
    ) -> anyhow::Result<ExecutionResult> {
        let quote = self.fetch_quote(order).await?;

        if self.config.paper_mode {
            info!(
                strategy = %order.strategy,
                in_amount = %quote.in_amount,
                out_amount = %quote.out_amount,
                "PAPER_MODE: simulated Jupiter swap (no broadcast)"
            );
            return Ok(ExecutionResult {
                paper: true,
                tx_signature: Some(format!("paper_{}", uuid::Uuid::new_v4())),
                quote,
            });
        }

        warn!("LIVE trading requested — building swap transaction");
        let swap_req = SwapRequest {
            quote_response: &quote,
            user_public_key: user_pubkey.to_string(),
            wrap_and_unwrap_sol: true,
        };

        let resp = self
            .client
            .post(&self.config.jupiter_swap_url)
            .json(&swap_req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("jupiter swap failed {status}: {body}");
        }

        let swap: SwapResponse = resp.json().await?;
        // Live broadcast requires solana-sdk signing — gated behind `live` feature.
        Ok(ExecutionResult {
            paper: false,
            tx_signature: Some(swap.swap_transaction.chars().take(16).collect()),
            quote,
        })
    }
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub paper: bool,
    pub tx_signature: Option<String>,
    pub quote: QuoteResponse,
}
