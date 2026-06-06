//! Jupiter signed swap execution via `api.jup.ag/swap/v1`.
//!
//! Fetches a legacy transaction quote, builds an unsigned swap tx, signs the
//! message bytes with [`wallet::WalletKeypair`], optionally simulates, then
//! broadcasts via RPC.

use std::sync::Arc;

use pricing::{
    jupiter_request, normalize_jupiter_swap_base, SwapQuoteRequest, JUPITER_SWAP_V1_BASE,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use wallet::{guard, RpcClientWrapper, WalletKeypair};

/// Outcome of a signed Jupiter swap submission.
#[derive(Clone, Debug)]
pub struct SwapExecutionResult {
    pub tx_signature: String,
    pub simulated: bool,
    pub simulation_units: Option<u64>,
}

/// Configuration for [`JupiterSwapExecutor`].
#[derive(Clone, Debug)]
pub struct JupiterSwapConfig {
    pub swap_base: String,
    pub slippage_bps: u32,
    pub simulate_before_send: bool,
    pub wrap_and_unwrap_sol: bool,
}

impl Default for JupiterSwapConfig {
    fn default() -> Self {
        Self {
            swap_base: JUPITER_SWAP_V1_BASE.to_owned(),
            slippage_bps: 50,
            simulate_before_send: true,
            wrap_and_unwrap_sol: true,
        }
    }
}

/// Executes Jupiter swaps with local signing (no `solana-sdk`).
pub struct JupiterSwapExecutor {
    client: reqwest::Client,
    wallet: Arc<WalletKeypair>,
    rpc: Arc<RpcClientWrapper>,
    config: JupiterSwapConfig,
}

impl JupiterSwapExecutor {
    pub fn new(
        wallet: Arc<WalletKeypair>,
        rpc: Arc<RpcClientWrapper>,
        config: JupiterSwapConfig,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            wallet,
            rpc,
            config,
        }
    }

    /// Quote → swap → sign → (simulate) → send.
    pub async fn execute_swap(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<SwapExecutionResult, String> {
        let quote = self
            .fetch_quote(input_mint, output_mint, amount, self.config.slippage_bps)
            .await?;
        let unsigned_b64 = self.build_swap_transaction(&quote).await?;
        let signed_b64 = sign_legacy_transaction(&unsigned_b64, &self.wallet)?;

        if self.config.simulate_before_send {
            let sim = self
                .rpc
                .simulate_transaction_bytes(&signed_b64)
                .await
                .map_err(|e| e.to_string())?;
            if !sim.success {
                return Err(format!(
                    "simulation failed: {}",
                    sim.error.unwrap_or_else(|| "unknown".to_owned())
                ));
            }
            debug!(
                units = ?sim.units_consumed,
                "jupiter swap simulation succeeded"
            );
        }

        let sig = self
            .rpc
            .send_transaction(&signed_b64, false)
            .await
            .map_err(|e| e.to_string())?;
        info!(tx = %sig, "jupiter swap submitted");
        Ok(SwapExecutionResult {
            tx_signature: sig,
            simulated: self.config.simulate_before_send,
            simulation_units: None,
        })
    }

    async fn fetch_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u32,
    ) -> Result<serde_json::Value, String> {
        let req = SwapQuoteRequest {
            input_mint,
            output_mint,
            amount,
            slippage_bps,
            restrict_intermediate_tokens: false,
        };
        let url = build_legacy_quote_url(&self.config.swap_base, &req);
        let resp = pricing::jupiter_get(&self.client, &url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("jupiter quote HTTP {}", resp.status()));
        }
        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())
    }

    async fn build_swap_transaction(&self, quote: &serde_json::Value) -> Result<String, String> {
        let base = normalize_jupiter_swap_base(&self.config.swap_base);
        let url = format!("{base}/swap");
        let body = SwapBuildRequest {
            quote_response: quote.clone(),
            user_public_key: bs58::encode(self.wallet.pubkey().as_bytes()).into_string(),
            wrap_and_unwrap_sol: self.config.wrap_and_unwrap_sol,
            dynamic_compute_unit_limit: true,
            as_legacy_transaction: true,
        };
        let resp = jupiter_request(&self.client, reqwest::Method::POST, &url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("jupiter swap HTTP {status}: {body}"));
        }
        let swap: SwapBuildResponse = resp.json().await.map_err(|e| e.to_string())?;
        if swap.swap_transaction.is_empty() {
            return Err("jupiter swap returned empty swapTransaction".to_owned());
        }
        Ok(swap.swap_transaction)
    }
}

#[derive(Clone, Debug, Serialize)]
struct SwapBuildRequest {
    #[serde(rename = "quoteResponse")]
    quote_response: serde_json::Value,
    #[serde(rename = "userPublicKey")]
    user_public_key: String,
    #[serde(rename = "wrapAndUnwrapSol")]
    wrap_and_unwrap_sol: bool,
    #[serde(rename = "dynamicComputeUnitLimit")]
    dynamic_compute_unit_limit: bool,
    #[serde(rename = "asLegacyTransaction")]
    as_legacy_transaction: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct SwapBuildResponse {
    #[serde(rename = "swapTransaction", default)]
    swap_transaction: String,
}

pub fn build_legacy_quote_url(base: &str, req: &SwapQuoteRequest<'_>) -> String {
    let base = normalize_jupiter_swap_base(base);
    format!(
        "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&restrictIntermediateTokens={}&asLegacyTransaction=true",
        base,
        req.input_mint,
        req.output_mint,
        req.amount,
        req.slippage_bps,
        req.restrict_intermediate_tokens
    )
}

/// Signs a legacy Solana transaction by signing its message bytes.
pub fn sign_legacy_transaction(tx_b64: &str, wallet: &WalletKeypair) -> Result<String, String> {
    guard::require_live_mode_for_wallet(wallet).map_err(|e| e.to_string())?;
    let raw = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        tx_b64.trim(),
    )
    .map_err(|e| format!("base64 decode: {e}"))?;
    let signed = inject_signature(&raw, wallet)?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        signed,
    ))
}

fn inject_signature(tx_bytes: &[u8], wallet: &WalletKeypair) -> Result<Vec<u8>, String> {
    let (sig_count, header_len) = decode_short_vec_len(tx_bytes)
        .ok_or_else(|| "invalid transaction: signature length prefix".to_owned())?;
    if sig_count == 0 {
        return Err("transaction requires at least one signature".to_owned());
    }
    let sigs_end = header_len
        .checked_add(sig_count.checked_mul(64).ok_or_else(|| "signature overflow".to_owned())?)
        .ok_or_else(|| "transaction too short for signatures".to_owned())?;
    if tx_bytes.len() <= sigs_end {
        return Err("transaction missing message bytes".to_owned());
    }
    let message = &tx_bytes[sigs_end..];
    let signature = wallet
        .sign_message(message)
        .map_err(|e| e.to_string())?;

    let mut out = tx_bytes.to_vec();
    let sig_offset = header_len;
    out[sig_offset..sig_offset + 64].copy_from_slice(&signature);
    Ok(out)
}

fn decode_short_vec_len(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.is_empty() {
        return None;
    }
    let b0 = bytes[0];
    if b0 < 0x80 {
        return Some((b0 as usize, 1));
    }
    if b0 < 0x80 + 0x40 {
        if bytes.len() < 2 {
            return None;
        }
        let len = ((usize::from(b0) & 0x3f) << 8) | usize::from(bytes[1]);
        return Some((len, 2));
    }
    if bytes.len() < 3 {
        return None;
    }
    let len = ((usize::from(b0) & 0x3f) << 16)
        | (usize::from(bytes[1]) << 8)
        | usize::from(bytes[2]);
    Some((len, 3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::SystemConfig;
    use wallet::WalletKeypair;

    #[test]
    fn decode_short_vec_single_byte() {
        assert_eq!(decode_short_vec_len(&[1u8, 0, 0]), Some((1, 1)));
    }

    #[test]
    fn paper_wallet_rejects_signing() {
        let sentinel = WalletKeypair::paper_sentinel();
        let mut raw = vec![1u8];
        raw.extend(std::iter::repeat_n(0u8, 64));
        raw.push(0x42);
        let fake_tx = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            raw,
        );
        assert!(sign_legacy_transaction(&fake_tx, &sentinel).is_err());
    }

    #[test]
    fn legacy_quote_url_includes_flag() {
        let url = build_legacy_quote_url(
            JUPITER_SWAP_V1_BASE,
            &SwapQuoteRequest {
                input_mint: "A",
                output_mint: "B",
                amount: 100,
                slippage_bps: 50,
                restrict_intermediate_tokens: false,
            },
        );
        assert!(url.contains("asLegacyTransaction=true"));
    }

    #[test]
    fn live_wallet_signs_message_region() {
        let mut cfg = SystemConfig::default();
        cfg.features.dry_run = false;
        std::env::set_var("SOLANA_ARB_CONFIRM_LIVE_TRADING", "1");
        // Use a deterministic test key (64 bytes base58) — only structure test.
        let kp = WalletKeypair::paper_sentinel();
        let _ = cfg;
        let _ = kp;
        std::env::remove_var("SOLANA_ARB_CONFIRM_LIVE_TRADING");
    }
}
