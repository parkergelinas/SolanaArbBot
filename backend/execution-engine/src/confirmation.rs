//! Async on-chain confirmation with real `getSignatureStatuses` polling.

use anyhow::{anyhow, Result};
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, warn};

const POLL_INTERVAL_MS: u64 = 400;
const TIMEOUT_SECS: u64 = 30;
const MAX_RETRIES: u32 = 3;

pub struct ConfirmationService {
    pub paper_mode: bool,
    rpc_endpoint: String,
    http: reqwest::Client,
}

impl ConfirmationService {
    pub fn new(paper_mode: bool) -> Self {
        let rpc_endpoint = std::env::var("SOLANA_RPC_ENDPOINT")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into());
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            paper_mode,
            rpc_endpoint,
            http,
        }
    }

    /// Polls `getSignatureStatuses` every 400 ms for up to 30 seconds.
    ///
    /// Returns `Ok(true)` when the transaction reaches `"confirmed"` or
    /// `"finalized"` commitment, `Ok(false)` on timeout, or `Err` when the RPC
    /// consistently fails after 3 retries with 200 ms exponential back-off.
    pub async fn confirm(&self, signature: &str) -> Result<bool> {
        if self.paper_mode {
            debug!(signature, "paper confirmation (instant)");
            return Ok(true);
        }

        let deadline = Instant::now() + Duration::from_secs(TIMEOUT_SECS);

        loop {
            if Instant::now() >= deadline {
                debug!(signature, "confirmation timed out after {}s", TIMEOUT_SECS);
                return Ok(false);
            }

            match self.poll_with_retry(signature).await {
                Ok(true) => return Ok(true),
                Ok(false) => {}
                Err(e) => return Err(e),
            }

            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
    }

    /// Calls `getSignatureStatuses` up to `MAX_RETRIES` times with 200 ms
    /// exponential backoff.  Returns `Ok(true)` when confirmed, `Ok(false)`
    /// when not yet landed, `Err` after all retries exhausted.
    async fn poll_with_retry(&self, signature: &str) -> Result<bool> {
        let mut delay_ms: u64 = 200;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                sleep(Duration::from_millis(delay_ms)).await;
                delay_ms *= 2;
            }

            match self.get_status(signature).await {
                Ok(result) => return Ok(result),
                Err(e) if attempt + 1 == MAX_RETRIES => {
                    warn!(signature, error = %e, "RPC signature-status failed after all retries");
                    return Err(e);
                }
                Err(e) => {
                    warn!(signature, attempt, error = %e, "RPC signature-status transient error");
                }
            }
        }

        Err(anyhow!("exhausted retries for signature {signature}"))
    }

    async fn get_status(&self, signature: &str) -> Result<bool> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id":      1,
            "method":  "getSignatureStatuses",
            "params":  [[signature], {"searchTransactionHistory": true}]
        });

        let resp: serde_json::Value = self
            .http
            .post(&self.rpc_endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("http send: {e}"))?
            .json()
            .await
            .map_err(|e| anyhow!("json parse: {e}"))?;

        if let Some(err) = resp.get("error") {
            return Err(anyhow!("rpc error: {err}"));
        }

        let slot = &resp["result"]["value"][0];
        if slot.is_null() {
            return Ok(false); // not yet on-chain
        }

        let status = slot["confirmationStatus"].as_str().unwrap_or("");
        Ok(matches!(status, "confirmed" | "finalized"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paper_service() -> ConfirmationService {
        ConfirmationService {
            paper_mode: true,
            rpc_endpoint: "http://localhost:8899".into(),
            http: reqwest::Client::new(),
        }
    }

    #[tokio::test]
    async fn paper_mode_returns_true_immediately() {
        let svc = paper_service();
        let result = svc.confirm("anySig").await.expect("no error");
        assert!(result);
    }

    #[tokio::test]
    async fn rpc_failure_after_retries_returns_err() {
        // Point at a closed port — every attempt will fail, exercising retry logic.
        let svc = ConfirmationService {
            paper_mode: false,
            rpc_endpoint: "http://127.0.0.1:1".into(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_millis(50))
                .build()
                .unwrap(),
        };
        let result = svc.confirm("badSig").await;
        assert!(result.is_err(), "expected Err after RPC failures");
    }
}
