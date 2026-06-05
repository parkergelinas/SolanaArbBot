//! Jupiter execution orchestration with quote cache.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::info;

use super::client::{JupiterClient, QuoteResponse, SwapRequest, SwapResponse};
use crate::config::EngineConfig;
use crate::strategy::SizedOrder;

#[derive(Clone)]
struct CacheEntry {
    quote: QuoteResponse,
    inserted: Instant,
}

pub struct JupiterExecutor {
    client: Arc<JupiterClient>,
    config: EngineConfig,
    cache: DashMap<String, CacheEntry>,
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub paper: bool,
    pub tx_signature: Option<String>,
    pub quote: QuoteResponse,
}

impl JupiterExecutor {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            client: Arc::new(JupiterClient::new(&config)),
            config,
            cache: DashMap::new(),
        }
    }

    pub fn client(&self) -> Arc<JupiterClient> {
        self.client.clone()
    }

    pub async fn prewarm(&self) -> anyhow::Result<()> {
        self.client.prewarm().await?;
        Ok(())
    }

    fn cache_key(order: &SizedOrder) -> String {
        format!(
            "{}:{}:{}",
            order.input_mint, order.output_mint, order.amount_lamports
        )
    }

    pub async fn get_quote(&self, order: &SizedOrder) -> anyhow::Result<QuoteResponse> {
        let key = Self::cache_key(order);
        let ttl = Duration::from_millis(self.config.quote_cache_ttl_ms);
        if let Some(entry) = self.cache.get(&key) {
            if entry.inserted.elapsed() < ttl {
                return Ok(entry.quote.clone());
            }
        }

        let req = JupiterClient::build_quote_request(order, 0);
        let quote = self.client.get_quote(&req).await?;
        self.cache.insert(
            key,
            CacheEntry {
                quote: quote.clone(),
                inserted: Instant::now(),
            },
        );
        Ok(quote)
    }

    pub async fn execute(&self, order: &SizedOrder) -> anyhow::Result<ExecutionResult> {
        let quote = self.get_quote(order).await?;
        self.execute_with_quote(order, quote).await
    }

    pub async fn execute_with_quote(
        &self,
        order: &SizedOrder,
        quote: QuoteResponse,
    ) -> anyhow::Result<ExecutionResult> {
        if self.config.paper_mode {
            info!(
                strategy = %order.strategy,
                in_amount = %quote.in_amount,
                out_amount = %quote.out_amount,
                "PAPER_MODE: quote obtained, no broadcast"
            );
            return Ok(ExecutionResult {
                paper: true,
                tx_signature: Some(format!("paper_{}", uuid::Uuid::new_v4())),
                quote,
            });
        }

        let swap_req = SwapRequest {
            quote_response: quote.clone(),
            user_public_key: self.config.wallet_pubkey.clone(),
            wrap_and_unwrap_sol: true,
            dynamic_compute_unit_limit: true,
        };
        let swap: SwapResponse = self.client.get_swap_tx(&swap_req).await?;
        Ok(ExecutionResult {
            paper: false,
            tx_signature: Some(swap.swap_transaction.chars().take(44).collect()),
            quote,
        })
    }
}
