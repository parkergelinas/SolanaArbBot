//! Polymarket HTTP client — shared by both the scanner and backtest binaries.
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use tokio::sync::Semaphore;
use tracing::info;

use crate::types::{GammaMarket, LastTradePrice, Midpoint, OrderBook, Trade};

const CLOB_BASE: &str = "https://clob.polymarket.com";
const GAMMA_BASE: &str = "https://gamma-api.polymarket.com";
const DATA_BASE: &str = "https://data-api.polymarket.com";

/// Maximum concurrent in-flight CLOB requests.  Can be overridden via
/// `POLY_CONCURRENCY` env var.
fn default_concurrency() -> usize {
    std::env::var("POLY_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6)
        .clamp(1, 20)
}

pub struct PolyClient {
    http: Client,
    api_key: Option<String>,
    /// Guards the total number of concurrent CLOB requests.
    sem: Arc<Semaphore>,
}

impl PolyClient {
    pub fn new(api_key: Option<String>) -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent("polymarket-scanner/0.1")
            .build()
            .context("build reqwest client")?;
        let sem = Arc::new(Semaphore::new(default_concurrency()));
        Ok(Self { http, api_key, sem })
    }

    /// Clone the semaphore handle so callers can share the rate-limit pool.
    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.sem.clone()
    }

    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => req.header("POLY_API_KEY", key),
            None => req,
        }
    }

    /// Execute a request with a per-call timeout, returning the deserialized body.
    ///
    /// Acquires one semaphore permit for the duration of the HTTP round-trip so
    /// callers that run many requests in parallel stay within `POLY_CONCURRENCY`.
    /// Always applies the API key header when one is configured (no-op otherwise).
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
        timeout_secs: u64,
        label: &'static str,
    ) -> Result<T> {
        let _permit = self
            .sem
            .acquire()
            .await
            .context("semaphore closed")?;

        // Apply auth header when an API key is configured; otherwise no-op.
        let req = self.authed(req);

        let fut = async move {
            req.send()
                .await
                .with_context(|| format!("GET {label}"))?
                .error_for_status()
                .with_context(|| format!("{label} HTTP error"))?
                .json::<T>()
                .await
                .with_context(|| format!("parse {label} JSON"))
        };

        tokio::time::timeout(Duration::from_secs(timeout_secs), fut)
            .await
            .with_context(|| format!("{label} timed out after {timeout_secs}s"))?
    }

    // ── Gamma API ─────────────────────────────────────────────────────────────

    /// Fetch all active, open, order-book-enabled markets from Gamma, paginated.
    pub async fn fetch_gamma_markets(&self) -> Result<Vec<GammaMarket>> {
        let mut all: Vec<GammaMarket> = Vec::new();
        let mut offset = 0usize;
        let limit = 100usize;

        loop {
            let req = self.http.get(format!("{GAMMA_BASE}/markets")).query(&[
                ("active", "true"),
                ("closed", "false"),
                ("enableOrderBook", "true"),
                ("order", "volume24hr"),
                ("ascending", "false"),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ]);
            let page: Vec<GammaMarket> =
                self.get_json(req, 30, "gamma /markets").await?;
            let n = page.len();
            all.extend(page);
            if n < limit {
                break;
            }
            offset += limit;
        }

        Ok(all)
    }

    /// Fetch resolved (closed) markets, keeping only clear binary resolutions.
    pub async fn fetch_resolved_markets(&self, page_limit: usize) -> Result<Vec<GammaMarket>> {
        let mut all = Vec::new();
        let limit = 100usize;

        for page in 0..page_limit {
            let offset = page * limit;
            let req = self.http.get(format!("{GAMMA_BASE}/markets")).query(&[
                ("closed", "true"),
                ("enableOrderBook", "true"),
                ("order", "closedTime"),
                ("ascending", "false"),
                ("limit", &limit.to_string()),
                ("offset", &offset.to_string()),
            ]);
            let batch: Vec<GammaMarket> = match self
                .get_json(req, 30, "gamma /markets (closed)")
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("resolved markets page {page} failed: {e}");
                    break;
                }
            };
            let n = batch.len();
            let resolved: Vec<GammaMarket> = batch
                .into_iter()
                .filter(|m| {
                    m.outcome_prices
                        .as_deref()
                        .map_or(false, |prices| prices.iter().any(|p| p == "1"))
                })
                .collect();
            info!(
                page,
                fetched = n,
                resolved = resolved.len(),
                "resolved market page"
            );
            all.extend(resolved);
            if n < limit {
                break;
            }
        }

        Ok(all)
    }

    // ── CLOB API (public endpoints — no auth required) ────────────────────────

    pub async fn fetch_book(&self, token_id: &str) -> Result<OrderBook> {
        let req = self
            .http
            .get(format!("{CLOB_BASE}/book"))
            .query(&[("token_id", token_id)]);
        self.get_json(req, 10, "/book").await
    }

    pub async fn fetch_midpoint(&self, token_id: &str) -> Result<Midpoint> {
        let req = self
            .http
            .get(format!("{CLOB_BASE}/midpoint"))
            .query(&[("token_id", token_id)]);
        self.get_json(req, 8, "/midpoint").await
    }

    pub async fn fetch_last_trade(&self, token_id: &str) -> Result<LastTradePrice> {
        let req = self
            .http
            .get(format!("{CLOB_BASE}/last-trade-price"))
            .query(&[("token_id", token_id)]);
        self.get_json(req, 8, "/last-trade-price").await
    }

    // ── Data API ──────────────────────────────────────────────────────────────

    /// Recent trades for a market (up to `limit`, max 500 per call).
    pub async fn fetch_trades(&self, condition_id: &str, limit: usize) -> Result<Vec<Trade>> {
        let req = self
            .http
            .get(format!("{DATA_BASE}/trades"))
            .query(&[("market", condition_id), ("limit", &limit.to_string())]);
        self.get_json(req, 15, "data-api /trades").await
    }
}
