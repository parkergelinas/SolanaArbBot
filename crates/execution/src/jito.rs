//! Jito bundle construction and parallel endpoint submission.
//!
//! Transaction bytes are stubbed until a signing keypair is wired.  Bundle
//! structure (tip tx + swap tx, versioned + ALT) is modeled here.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use config::ArbitrageConfig;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Official Jito block-engine bundle endpoints (US / EU / Asia).
pub const JITO_ENDPOINTS: &[&str] = &[
    "https://mainnet.block-engine.jito.wtf/api/v1/bundles",
    "https://amsterdam.mainnet.block-engine.jito.wtf/api/v1/bundles",
    "https://tokyo.mainnet.block-engine.jito.wtf/api/v1/bundles",
];

/// Jito tip accounts (subset of the 8 official accounts).
pub const JITO_TIP_ACCOUNTS: &[&str] = &[
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "HFqU5x63VTqvQss8hp1iYwxTxh6b2m1YpF5xJ8vZgK9r",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgopwjb2DTLSoktnz8dwnrap7Q6Qv",
];

/// Compute-unit limit for arb bundles.
pub const COMPUTE_UNITS_LIMIT: u32 = 300_000;

/// Solana commitment for bundle landing checks — **processed**, not confirmed.
pub const JITO_COMMITMENT: &str = "processed";

static BUNDLE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Bundle submission request from the arbitrage engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleRequest {
    pub opportunity_id: String,
    pub tip_lamports: u64,
    pub priority_fee_lamports: u64,
    pub amount_in_lamports: u64,
    pub route_hops: usize,
}

/// Pre-built bundle ready for submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltBundle {
    pub tip_tx: Vec<u8>,
    pub swap_tx: Vec<u8>,
    pub compute_unit_price: u64,
}

/// Result of a bundle submission attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleSubmitResult {
    pub bundle_id: String,
    pub endpoint: String,
    pub landed: bool,
    pub error: Option<String>,
}

/// Jito JSON-RPC response envelope.
#[derive(Debug, Deserialize)]
struct JitoResponse {
    result: Option<String>,
    error: Option<JitoError>,
}

#[derive(Debug, Deserialize)]
struct JitoError {
    message: String,
}

#[derive(Serialize)]
struct JitoRequest<'a> {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: Vec<Vec<&'a str>>,
}

/// Jito bundle submitter.
#[derive(Clone, Debug)]
pub struct JitoSubmitter {
    endpoints: Vec<String>,
    parallel: bool,
    commitment: String,
    http: reqwest::Client,
}

impl JitoSubmitter {
    /// Creates a submitter from arbitrage configuration.
    #[must_use]
    pub fn new(cfg: &ArbitrageConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            endpoints: JITO_ENDPOINTS.iter().map(|s| (*s).to_owned()).collect(),
            parallel: cfg.parallel_endpoints,
            commitment: cfg.commitment.clone(),
            http,
        }
    }

    /// Builds a two-transaction bundle (tip + atomic swap).
    ///
    /// Real on-chain tx construction requires a loaded keypair — stub bytes are
    /// emitted until wallet integration is complete.
    #[must_use]
    pub fn build_bundle(&self, req: &BundleRequest) -> BuiltBundle {
        let cu_price = req
            .priority_fee_lamports
            .saturating_mul(1_000_000)
            / u64::from(COMPUTE_UNITS_LIMIT);

        let tip_account = JITO_TIP_ACCOUNTS
            [(req.amount_in_lamports as usize) % JITO_TIP_ACCOUNTS.len()];

        // Stub serialized transactions — replaced by versioned tx + ALT builder.
        let tip_tx = format!(
            "TIP:v1|acct={tip_account}|lamports={}|commitment={}",
            req.tip_lamports, self.commitment
        )
        .into_bytes();

        let swap_tx = format!(
            "SWAP:v0|hops={}|amount={}|cu_limit={}|cu_price={}|alt=1",
            req.route_hops, req.amount_in_lamports, COMPUTE_UNITS_LIMIT, cu_price
        )
        .into_bytes();

        BuiltBundle {
            tip_tx,
            swap_tx,
            compute_unit_price: cu_price,
        }
    }

    /// Submits a bundle to Jito endpoints (parallel when configured).
    pub async fn submit_bundle(&self, req: BundleRequest) -> common::Result<BundleSubmitResult> {
        let bundle = self.build_bundle(&req);
        let encoded: Vec<String> = [bundle.tip_tx, bundle.swap_tx]
            .iter()
            .map(|tx| base64_encode(tx))
            .collect();

        if self.parallel {
            self.submit_parallel(&encoded, &req.opportunity_id).await
        } else {
            self.submit_single(&self.endpoints[0], &encoded, &req.opportunity_id)
                .await
        }
    }

    async fn submit_parallel(
        &self,
        txs: &[String],
        opportunity_id: &str,
    ) -> common::Result<BundleSubmitResult> {
        let id = BUNDLE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let params: Vec<&str> = txs.iter().map(String::as_str).collect();

        let post = |endpoint: String| {
            let client = self.http.clone();
            let params = params.clone();
            async move {
                let result = Self::post_bundle(&client, &endpoint, id, &params).await;
                (endpoint, result)
            }
        };

        let (r0, r1, r2) = tokio::join!(
            post(self.endpoints[0].clone()),
            post(self.endpoints[1].clone()),
            post(self.endpoints[2].clone()),
        );

        for (ep, result) in [r0, r1, r2] {
            match result {
                Ok(bundle_id) => {
                    debug!(
                        opportunity_id,
                        endpoint = %ep,
                        bundle_id = %bundle_id,
                        commitment = JITO_COMMITMENT,
                        "bundle accepted"
                    );
                    return Ok(BundleSubmitResult {
                        bundle_id,
                        endpoint: ep,
                        landed: true,
                        error: None,
                    });
                }
                Err(e) => {
                    debug!(endpoint = %ep, error = %e, "bundle endpoint failed");
                }
            }
        }

        Ok(BundleSubmitResult {
            bundle_id: String::new(),
            endpoint: String::new(),
            landed: false,
            error: Some("all Jito endpoints rejected bundle".into()),
        })
    }

    async fn submit_single(
        &self,
        endpoint: &str,
        txs: &[String],
        opportunity_id: &str,
    ) -> common::Result<BundleSubmitResult> {
        let id = BUNDLE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let params: Vec<&str> = txs.iter().map(String::as_str).collect();

        match Self::post_bundle(&self.http, endpoint, id, &params).await {
            Ok(bundle_id) => {
                debug!(opportunity_id, endpoint, bundle_id = %bundle_id, "bundle accepted");
                Ok(BundleSubmitResult {
                    bundle_id,
                    endpoint: endpoint.to_owned(),
                    landed: true,
                    error: None,
                })
            }
            Err(e) => {
                warn!(opportunity_id, endpoint, error = %e, "bundle rejected");
                Ok(BundleSubmitResult {
                    bundle_id: String::new(),
                    endpoint: endpoint.to_owned(),
                    landed: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    async fn post_bundle(
        client: &reqwest::Client,
        endpoint: &str,
        id: u64,
        txs: &[&str],
    ) -> Result<String, String> {
        let body = JitoRequest {
            jsonrpc: "2.0",
            id,
            method: "sendBundle",
            params: vec![txs.to_vec()],
        };

        let resp = client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("http error: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("http status {}", resp.status()));
        }

        let parsed: JitoResponse = resp
            .json()
            .await
            .map_err(|e| format!("json parse error: {e}"))?;

        if let Some(err) = parsed.error {
            return Err(err.message);
        }

        parsed
            .result
            .ok_or_else(|| "empty bundle id in response".to_owned())
    }
}

/// Blocking wrapper for the sync cold-path thread.
pub fn submit_bundle_blocking(
    submitter: &JitoSubmitter,
    req: BundleRequest,
) -> common::Result<BundleSubmitResult> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| common::Error::InternalError(format!("tokio runtime: {e}")))?;

    rt.block_on(submitter.submit_bundle(req))
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::ArbitrageConfig;

    #[test]
    fn build_bundle_produces_tip_and_swap_txs() {
        let submitter = JitoSubmitter::new(&ArbitrageConfig::default());
        let req = BundleRequest {
            opportunity_id: "test".into(),
            tip_lamports: 50_000,
            priority_fee_lamports: 100_000,
            amount_in_lamports: 1_000_000_000,
            route_hops: 2,
        };
        let bundle = submitter.build_bundle(&req);
        assert!(!bundle.tip_tx.is_empty());
        assert!(!bundle.swap_tx.is_empty());
        assert!(bundle.compute_unit_price > 0);
    }

    #[test]
    fn compute_unit_price_scales_with_priority_fee() {
        let submitter = JitoSubmitter::new(&ArbitrageConfig::default());
        let req = BundleRequest {
            opportunity_id: "test".into(),
            tip_lamports: 5_000,
            priority_fee_lamports: 100_000,
            amount_in_lamports: 500_000_000,
            route_hops: 3,
        };
        let bundle = submitter.build_bundle(&req);
        let expected = 100_000 * 1_000_000 / u64::from(COMPUTE_UNITS_LIMIT);
        assert_eq!(bundle.compute_unit_price, expected);
    }

    #[test]
    fn jito_endpoints_match_spec() {
        assert_eq!(JITO_ENDPOINTS.len(), 3);
        assert!(JITO_ENDPOINTS[0].contains("mainnet.block-engine.jito.wtf"));
        assert!(JITO_ENDPOINTS[1].contains("amsterdam"));
        assert!(JITO_ENDPOINTS[2].contains("tokyo"));
    }

    #[test]
    fn commitment_is_processed() {
        assert_eq!(JITO_COMMITMENT, "processed");
        let cfg = ArbitrageConfig::default();
        assert_eq!(cfg.commitment, "processed");
    }
}
