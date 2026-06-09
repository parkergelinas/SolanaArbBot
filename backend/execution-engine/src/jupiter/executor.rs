//! Jupiter execution orchestration with quote cache.
//!
//! # Paper mode (default)
//! Always safe: no signing, no real transactions.
//!
//! # Live mode (requires `--features live-signing`)
//! Decodes Jupiter's base64 VersionedTransaction, signs as fee-payer, and
//! submits to Jito block-engine endpoints concurrently.
//! Requires OpenSSL (pulled in by `solana-sdk` → `solana-secp256r1-program`).
//! On Linux this is automatic; on Windows install via vcpkg or chocolatey.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::info;

#[cfg(not(feature = "live-signing"))]
use super::client::{JupiterClient, QuoteResponse};
#[cfg(feature = "live-signing")]
use super::client::{JupiterClient, QuoteResponse, SwapRequest, SwapResponse};
use crate::config::EngineConfig;
use crate::strategy::SizedOrder;

#[derive(Clone)]
struct CacheEntry {
    quote: QuoteResponse,
    inserted: Instant,
}

/// Wallet handle type — real keypair when `live-signing` is enabled, `()` otherwise.
#[cfg(feature = "live-signing")]
type WalletHandle = Option<Arc<wallet::WalletKeypair>>;
#[cfg(not(feature = "live-signing"))]
type WalletHandle = ();

pub struct JupiterExecutor {
    client: Arc<JupiterClient>,
    config: EngineConfig,
    cache: DashMap<String, CacheEntry>,
    wallet: WalletHandle,
    rpc_endpoint: String,
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub paper: bool,
    pub tx_signature: Option<String>,
    pub quote: QuoteResponse,
}

impl JupiterExecutor {
    /// Constructs a new executor.
    ///
    /// `wallet` is `None` in paper mode.  When `live-signing` feature is disabled,
    /// the wallet parameter is not available and the type collapses to `()`.
    pub fn new(config: EngineConfig, wallet: WalletHandle) -> Self {
        let rpc_endpoint = std::env::var("SOLANA_RPC_ENDPOINT")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into());
        Self {
            client: Arc::new(JupiterClient::new(&config)),
            config,
            cache: DashMap::new(),
            wallet,
            rpc_endpoint,
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

        // ── Live execution ────────────────────────────────────────────────────
        #[cfg(feature = "live-signing")]
        {
            return self.execute_live(order, quote).await;
        }

        // Without live-signing feature, we cannot execute on-chain.
        #[cfg(not(feature = "live-signing"))]
        {
            Err(anyhow::anyhow!(
                "EXECUTION_LIVE=true requires the `live-signing` feature. \
                 Rebuild with: cargo build -p execution-engine --features live-signing \
                 (requires OpenSSL; on Windows: choco install openssl)"
            ))
        }
    }

    // ── Live-signing path ─────────────────────────────────────────────────────

    #[cfg(feature = "live-signing")]
    async fn execute_live(
        &self,
        order: &SizedOrder,
        quote: QuoteResponse,
    ) -> anyhow::Result<ExecutionResult> {
        use base64::Engine as _;

        let wallet = self
            .wallet
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("live mode requires a loaded wallet keypair"))?;

        let swap_req = SwapRequest {
            quote_response: quote.clone(),
            user_public_key: self.config.wallet_pubkey.clone(),
            wrap_and_unwrap_sol: true,
            dynamic_compute_unit_limit: true,
        };
        let swap: SwapResponse = self.client.get_swap_tx(&swap_req).await?;

        // Decode Jupiter's base64-encoded VersionedTransaction.
        let swap_bytes = base64::engine::general_purpose::STANDARD
            .decode(&swap.swap_transaction)
            .map_err(|e| anyhow::anyhow!("base64 decode swap tx: {e}"))?;

        let versioned_tx: solana_sdk::transaction::VersionedTransaction =
            bincode::deserialize(&swap_bytes)
                .map_err(|e| anyhow::anyhow!("bincode deserialize swap tx: {e}"))?;

        // Sign as fee-payer.
        let signed_tx = wallet.sign_transaction(versioned_tx);

        // Fetch a fresh blockhash for the Jito tip transfer.
        let blockhash = fetch_recent_blockhash(&self.rpc_endpoint).await?;

        let tip_lamports: u64 = std::env::var("JITO_TIP_LAMPORTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50_000);

        let bundle_uuid =
            submit_jito_bundle(&signed_tx, tip_lamports, wallet, &blockhash).await?;

        info!(bundle_uuid = %bundle_uuid, strategy = %order.strategy, "bundle submitted to Jito");

        Ok(ExecutionResult {
            paper: false,
            tx_signature: Some(bundle_uuid),
            quote,
        })
    }
}

// ── Jito bundle helpers (live-signing only) ───────────────────────────────────

#[cfg(feature = "live-signing")]
const JITO_TIP_ACCOUNT: &str = "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5";

#[cfg(feature = "live-signing")]
fn jito_ny_endpoint() -> String {
    std::env::var("JITO_BLOCK_ENGINE_NY")
        .unwrap_or_else(|_| "https://mainnet.block-engine.jito.wtf/api/v1/bundles".into())
}

#[cfg(feature = "live-signing")]
fn jito_eu_endpoint() -> String {
    std::env::var("JITO_BLOCK_ENGINE_EU").unwrap_or_else(|_| {
        "https://amsterdam.mainnet.block-engine.jito.wtf/api/v1/bundles".into()
    })
}

#[cfg(feature = "live-signing")]
async fn submit_jito_bundle(
    tx: &solana_sdk::transaction::VersionedTransaction,
    tip_lamports: u64,
    wallet: &wallet::WalletKeypair,
    blockhash: &[u8; 32],
) -> anyhow::Result<String> {
    use base64::Engine as _;

    let payer: [u8; 32] = *wallet.pubkey().as_bytes();

    let tip_bytes = bs58::decode(JITO_TIP_ACCOUNT)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("tip account decode: {e}"))?;
    let mut tip_acct = [0u8; 32];
    tip_acct[..tip_bytes.len().min(32)].copy_from_slice(&tip_bytes[..tip_bytes.len().min(32)]);

    let tip_msg = build_tip_tx_message(&payer, &tip_acct, tip_lamports, blockhash);
    let sig = wallet
        .sign_message(&tip_msg)
        .map_err(|e| anyhow::anyhow!("tip sign: {e}"))?;
    let tip_tx_bytes = wrap_signed_tx(&tip_msg, &sig);

    let swap_tx_bytes =
        bincode::serialize(tx).map_err(|e| anyhow::anyhow!("swap tx serialize: {e}"))?;

    let tip_b64 = base64::engine::general_purpose::STANDARD.encode(&tip_tx_bytes);
    let swap_b64 = base64::engine::general_purpose::STANDARD.encode(&swap_tx_bytes);

    let http = reqwest::Client::builder()
        .timeout(Duration::from_millis(1_500))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let (ny_res, eu_res) = tokio::join!(
        tokio::time::timeout(
            Duration::from_millis(1_500),
            post_bundle(&http, &jito_ny_endpoint(), &[&tip_b64, &swap_b64]),
        ),
        tokio::time::timeout(
            Duration::from_millis(1_500),
            post_bundle(&http, &jito_eu_endpoint(), &[&tip_b64, &swap_b64]),
        ),
    );

    for (name, result) in [("NY", ny_res), ("EU", eu_res)] {
        match result {
            Ok(Ok(uuid)) => {
                info!(endpoint = name, uuid = %uuid, "Jito bundle accepted");
                return Ok(uuid);
            }
            Ok(Err(e)) => tracing::warn!(endpoint = name, error = %e, "bundle rejected"),
            Err(_) => tracing::warn!(endpoint = name, "bundle endpoint timed out"),
        }
    }

    Err(anyhow::anyhow!("all Jito endpoints rejected or timed out"))
}

#[cfg(feature = "live-signing")]
async fn post_bundle(
    http: &reqwest::Client,
    endpoint: &str,
    txs: &[&str],
) -> Result<String, String> {
    #[derive(serde::Serialize)]
    struct Req<'a> {
        jsonrpc: &'static str,
        id: u64,
        method: &'static str,
        params: Vec<Vec<&'a str>>,
    }
    #[derive(serde::Deserialize)]
    struct Resp {
        result: Option<String>,
        error: Option<RespErr>,
    }
    #[derive(serde::Deserialize)]
    struct RespErr {
        message: String,
    }

    let body = Req {
        jsonrpc: "2.0",
        id: 1,
        method: "sendBundle",
        params: vec![txs.to_vec()],
    };

    let resp = http
        .post(endpoint)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("http: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("http {}", resp.status()));
    }

    let parsed: Resp = resp.json().await.map_err(|e| format!("json: {e}"))?;
    if let Some(err) = parsed.error {
        return Err(err.message);
    }
    parsed.result.ok_or_else(|| "empty bundle id".into())
}

#[cfg(feature = "live-signing")]
fn build_tip_tx_message(
    payer: &[u8; 32],
    tip_account: &[u8; 32],
    lamports: u64,
    blockhash: &[u8; 32],
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(3 + 1 + 96 + 32 + 1 + 1 + 1 + 2 + 1 + 12);
    msg.push(1u8);
    msg.push(0u8);
    msg.push(1u8);
    msg.push(3u8);
    msg.extend_from_slice(payer);
    msg.extend_from_slice(tip_account);
    msg.extend_from_slice(&[0u8; 32]); // system program
    msg.extend_from_slice(blockhash);
    msg.push(1u8); // instruction count
    msg.push(2u8); // program_id_index = system program
    msg.push(2u8); // account-index count
    msg.push(0u8); // payer
    msg.push(1u8); // tip_account
    msg.push(12u8); // data length
    msg.extend_from_slice(&2u32.to_le_bytes()); // SystemInstruction::Transfer
    msg.extend_from_slice(&lamports.to_le_bytes());
    msg
}

#[cfg(feature = "live-signing")]
fn wrap_signed_tx(message: &[u8], signature: &[u8; 64]) -> Vec<u8> {
    let mut tx = Vec::with_capacity(1 + 64 + message.len());
    tx.push(1u8);
    tx.extend_from_slice(signature);
    tx.extend_from_slice(message);
    tx
}

#[cfg(feature = "live-signing")]
async fn fetch_recent_blockhash(rpc_endpoint: &str) -> anyhow::Result<[u8; 32]> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestBlockhash",
        "params": [{"commitment": "confirmed"}]
    });

    let resp: serde_json::Value = http
        .post(rpc_endpoint)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let b58 = resp["result"]["value"]["blockhash"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("blockhash missing in RPC response"))?;

    let decoded = bs58::decode(b58)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("blockhash base58 decode: {e}"))?;

    if decoded.len() != 32 {
        return Err(anyhow::anyhow!(
            "unexpected blockhash length: {}",
            decoded.len()
        ));
    }

    let mut out = [0u8; 32];
    out.copy_from_slice(&decoded);
    Ok(out)
}
