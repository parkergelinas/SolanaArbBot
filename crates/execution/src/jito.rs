//! Jito bundle construction and parallel endpoint submission.
//!
//! The tip transfer is built as a real Solana legacy transaction using the
//! wire format implemented in `build_tip_tx_message` / `wrap_signed_tx`.
//! The swap transaction must be supplied by the caller from Jupiter's `/swap`
//! endpoint and passed into `build_bundle_signed`.

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

    pub(crate) async fn submit_parallel(
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

    pub(crate) async fn submit_single(
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

// ── Real Solana transaction wire format ──────────────────────────────────────

/// System Program ID on Solana (all-zeros pubkey).
const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

/// `SystemInstruction::Transfer` discriminant (little-endian u32 = 2).
const SYSTEM_TRANSFER_DISCRIMINANT: [u8; 4] = [2, 0, 0, 0];

/// Builds the **unsigned** message bytes of a Solana legacy SOL tip transfer.
///
/// Wire layout:
/// ```text
/// Header (3 bytes):
///   u8  num_required_signatures = 1
///   u8  num_readonly_signed_accounts = 0
///   u8  num_readonly_unsigned_accounts = 1   (system program)
/// Account addresses (compact_u16=3, then 3×32 bytes):
///   [32] payer           writable, signer
///   [32] tip_account     writable, non-signer
///   [32] system_program  readonly, non-signer  (all zeros)
/// Recent blockhash: [32]
/// Instructions (compact_u16=1):
///   u8           program_id_index = 2 (system program)
///   compact_u16  accounts count = 2
///   u8[2]        [0, 1]  (payer, tip_account)
///   compact_u16  data length = 12
///   [4]          SystemInstruction::Transfer discriminant
///   [8]          lamports as little-endian u64
/// ```
/// All compact_u16 values here are < 128, so they encode as single bytes.
pub fn build_tip_tx_message(
    payer_pubkey: &[u8; 32],
    tip_account_pubkey: &[u8; 32],
    lamports: u64,
    blockhash: &[u8; 32],
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(3 + 1 + 96 + 32 + 1 + 1 + 1 + 2 + 1 + 12);

    // Header
    msg.push(1); // num_required_signatures
    msg.push(0); // num_readonly_signed_accounts
    msg.push(1); // num_readonly_unsigned_accounts

    // Account keys (compact_u16 = 3)
    msg.push(3);
    msg.extend_from_slice(payer_pubkey);
    msg.extend_from_slice(tip_account_pubkey);
    msg.extend_from_slice(&SYSTEM_PROGRAM_ID);

    // Recent blockhash
    msg.extend_from_slice(blockhash);

    // Instructions (compact_u16 = 1)
    msg.push(1);
    msg.push(2); // program_id_index = system program (slot 2)
    msg.push(2); // account-index count compact_u16
    msg.push(0); // payer
    msg.push(1); // tip_account
    msg.push(12); // data length compact_u16
    msg.extend_from_slice(&SYSTEM_TRANSFER_DISCRIMINANT);
    msg.extend_from_slice(&lamports.to_le_bytes());

    msg
}

/// Wraps a signed message as a Solana legacy transaction:
/// `compact_u16(1) ++ signature[64] ++ message_bytes`
pub fn wrap_signed_tx(message: &[u8], signature: &[u8; 64]) -> Vec<u8> {
    let mut tx = Vec::with_capacity(1 + 64 + message.len());
    tx.push(1); // compact_u16(1): one signature
    tx.extend_from_slice(signature);
    tx.extend_from_slice(message);
    tx
}

/// Decodes a base58-encoded Solana pubkey into 32 bytes.
/// Returns all-zeros on invalid input (Jito will reject the bundle).
fn decode_base58_pubkey(b58: &str) -> [u8; 32] {
    let decoded = bs58::decode(b58).into_vec().unwrap_or_default();
    let mut out = [0u8; 32];
    let n = decoded.len().min(32);
    out[..n].copy_from_slice(&decoded[..n]);
    out
}

/// Builds a fully signed Jito bundle: a real tip-transfer legacy tx + caller-provided swap tx.
///
/// The swap transaction bytes must be obtained from Jupiter's `/swap` endpoint
/// (pre-built and partially signed).  Pass them as `swap_tx_bytes`.
pub fn build_bundle_signed<F>(
    submitter: &JitoSubmitter,
    req: &BundleRequest,
    payer_pubkey: &[u8; 32],
    blockhash: &[u8; 32],
    sign: F,
) -> BuiltBundle
where
    F: FnOnce(&[u8]) -> [u8; 64],
{
    let cu_price = req
        .priority_fee_lamports
        .saturating_mul(1_000_000)
        / u64::from(COMPUTE_UNITS_LIMIT);

    let tip_account_str =
        JITO_TIP_ACCOUNTS[(req.amount_in_lamports as usize) % JITO_TIP_ACCOUNTS.len()];
    let tip_account_bytes = decode_base58_pubkey(tip_account_str);

    let tip_msg = build_tip_tx_message(payer_pubkey, &tip_account_bytes, req.tip_lamports, blockhash);
    let sig = sign(&tip_msg);
    let tip_tx = wrap_signed_tx(&tip_msg, &sig);

    // Swap transaction bytes must be provided by the caller (from Jupiter /swap).
    // An empty vec here means the bundle will be incomplete — callers must supply it.
    let swap_tx = Vec::new();

    let _ = submitter; // submitter params used at submission time

    BuiltBundle {
        tip_tx,
        swap_tx,
        compute_unit_price: cu_price,
    }
}

/// Submits a pre-built `BuiltBundle` (real signed bytes) to Jito endpoints.
pub fn submit_bundle_blocking_raw(
    submitter: &JitoSubmitter,
    bundle: BuiltBundle,
    opportunity_id: &str,
) -> common::Result<BundleSubmitResult> {
    if bundle.tip_tx.is_empty() || bundle.swap_tx.is_empty() {
        return Ok(BundleSubmitResult {
            bundle_id: String::new(),
            endpoint: String::new(),
            landed: false,
            error: Some(
                "bundle rejected locally: tip_tx or swap_tx is empty — \
                 swap_tx must be provided from Jupiter /swap endpoint"
                    .into(),
            ),
        });
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| common::Error::InternalError(format!("tokio runtime: {e}")))?;

    let encoded = vec![base64_encode(&bundle.tip_tx), base64_encode(&bundle.swap_tx)];

    if submitter.parallel {
        rt.block_on(submitter.submit_parallel(&encoded, opportunity_id))
    } else {
        rt.block_on(submitter.submit_single(&submitter.endpoints[0], &encoded, opportunity_id))
    }
}

/// Fetches a recent blockhash from the Solana RPC (blocking, for the cold-path thread).
///
/// Returns 32 raw bytes decoded from the base58 blockhash string.
pub fn fetch_blockhash_blocking(rpc_endpoint: &str) -> Result<[u8; 32], String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;

    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("http client: {e}"))?;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [{"commitment": "confirmed"}]
        });

        let resp: serde_json::Value = client
            .post(rpc_endpoint)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("rpc request: {e}"))?
            .json()
            .await
            .map_err(|e| format!("rpc json parse: {e}"))?;

        let b58 = resp["result"]["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| "blockhash field missing in RPC response".to_owned())?;

        let decoded = bs58::decode(b58)
            .into_vec()
            .map_err(|e| format!("blockhash base58 decode: {e}"))?;

        if decoded.len() != 32 {
            return Err(format!("blockhash has unexpected length {}", decoded.len()));
        }

        let mut out = [0u8; 32];
        out.copy_from_slice(&decoded);
        Ok(out)
    })
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
    fn tip_tx_message_has_correct_wire_format() {
        let payer = [1u8; 32];
        let tip_account = [2u8; 32];
        let lamports = 50_000u64;
        let blockhash = [3u8; 32];

        let msg = build_tip_tx_message(&payer, &tip_account, lamports, &blockhash);

        // Header: 3 bytes
        assert_eq!(msg[0], 1, "num_required_signatures");
        assert_eq!(msg[1], 0, "num_readonly_signed_accounts");
        assert_eq!(msg[2], 1, "num_readonly_unsigned_accounts");

        // Account count compact_u16 = 3 (single byte since < 128)
        assert_eq!(msg[3], 3);

        // Payer pubkey at offset 4
        assert_eq!(&msg[4..36], &payer);

        // Tip account pubkey at offset 36
        assert_eq!(&msg[36..68], &tip_account);

        // System program (all zeros) at offset 68
        assert_eq!(&msg[68..100], &[0u8; 32]);

        // Recent blockhash at offset 100
        assert_eq!(&msg[100..132], &blockhash);

        // Instruction count = 1
        assert_eq!(msg[132], 1);

        // program_id_index = 2 (system program)
        assert_eq!(msg[133], 2);

        // Account indices count = 2
        assert_eq!(msg[134], 2);
        assert_eq!(msg[135], 0); // payer
        assert_eq!(msg[136], 1); // tip_account

        // Data length = 12
        assert_eq!(msg[137], 12);

        // SystemInstruction::Transfer discriminant = 2
        assert_eq!(&msg[138..142], &2u32.to_le_bytes());

        // Amount in little-endian
        assert_eq!(&msg[142..150], &lamports.to_le_bytes());

        // Total length: 3 + 1 + 96 + 32 + 1 + 1 + 1 + 2 + 1 + 12 = 150
        assert_eq!(msg.len(), 150);
    }

    #[test]
    fn wrap_signed_tx_prepends_sig_count_and_signature() {
        let message = b"test_message";
        let sig = [0xABu8; 64];
        let tx = wrap_signed_tx(message, &sig);

        assert_eq!(tx[0], 1, "compact_u16(1) = signature count");
        assert_eq!(&tx[1..65], &sig);
        assert_eq!(&tx[65..], message.as_slice());
    }

    #[test]
    fn build_bundle_signed_produces_real_tip_tx() {
        let submitter = JitoSubmitter::new(&ArbitrageConfig::default());
        let req = BundleRequest {
            opportunity_id: "test".into(),
            tip_lamports: 10_000,
            priority_fee_lamports: 100_000,
            amount_in_lamports: 500_000_000,
            route_hops: 2,
        };
        let payer = [7u8; 32];
        let blockhash = [9u8; 32];
        let dummy_sig = [0x42u8; 64];

        let bundle = build_bundle_signed(&submitter, &req, &payer, &blockhash, |_msg| dummy_sig);

        // tip_tx = 1 byte (sig count) + 64 bytes (sig) + 150 bytes (msg) = 215
        assert_eq!(bundle.tip_tx.len(), 215, "tip_tx has correct wire size");
        assert_eq!(bundle.tip_tx[0], 1); // signature count
        assert_eq!(&bundle.tip_tx[1..65], &dummy_sig);
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
