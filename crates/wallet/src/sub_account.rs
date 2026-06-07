//! Trading sub-account — a dedicated hot-wallet for bot trade execution.
//!
//! # Architecture
//!
//! The bot never trades with the main wallet (the one connected in the
//! dashboard).  Instead a separate *trading sub-account* keypair — loaded from
//! `SOLANA_ARB_TRADING_KEY` — holds a capped amount of SOL and signs every swap.
//!
//! ```text
//!  ┌──────────────────────────────┐
//!  │  Main wallet (Phantom / UI)  │  ← user connects this in the dashboard
//!  │  SOLANA_ARB_WALLET_KEY       │
//!  └────────────┬─────────────────┘
//!               │  fund (SOL transfer)
//!               ▼
//!  ┌──────────────────────────────┐
//!  │  Trading sub-account         │  ← signs every bot swap / Jito bundle
//!  │  SOLANA_ARB_TRADING_KEY      │  max balance capped at sub_account.max_sol
//!  └────────────┬─────────────────┘
//!               │  sweep back on halt / session end
//!               └──────────────────────────────────►  main wallet
//! ```
//!
//! # Security properties
//!
//! * Maximum SOL exposure is `max_sol` — even a full compromise of the trading
//!   keypair loses at most that much.
//! * Sweep destination is locked to the main wallet pubkey at load time and
//!   never modified at runtime.
//! * Paper-mode sub-accounts are accepted without any env var requirement.
//! * No secret material is ever logged beyond the base58 public key.

use common::Pubkey;
use tracing::{info, warn};
use zeroize::Zeroizing;

use crate::error::{WalletError, WalletResult};
use crate::keypair::WalletKeypair;
use crate::rpc::RpcClientWrapper;
use crate::transfer::{send_sol_transfer, RENT_EXEMPT_MIN_LAMPORTS};

/// Minimum sweepable amount — skip sweep if below this (fees exceed value).
const MIN_SWEEP_LAMPORTS: u64 = 100_000; // 0.0001 SOL

// ── TradingSubAccount ─────────────────────────────────────────────────────────

/// A dedicated hot-wallet keypair for bot trade execution.
///
/// Funded by the main wallet up to `max_lamports`; sweeps profits back on halt.
pub struct TradingSubAccount {
    keypair: WalletKeypair,
    /// Funding source and sole sweep destination — fixed at construction.
    main_pubkey: Pubkey,
    max_lamports: u64,
}

impl std::fmt::Debug for TradingSubAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TradingSubAccount")
            .field("pubkey", &bs58::encode(self.keypair.pubkey().as_bytes()).into_string())
            .field("main_pubkey", &bs58::encode(self.main_pubkey.as_bytes()).into_string())
            .field("max_lamports", &self.max_lamports)
            .finish()
    }
}

impl TradingSubAccount {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Load the trading sub-account from the `SOLANA_ARB_TRADING_KEY` env var.
    ///
    /// The env var must contain a base58-encoded 64-byte Solana keypair (the
    /// same format produced by `solana-keygen new --outfile key.json` and then
    /// exported as base58).
    ///
    /// * `main_pubkey` — funding source and sweep destination (immutable).
    /// * `max_sol`     — SOL cap; the sub-account is never funded above this.
    pub fn load(main_pubkey: Pubkey, max_sol: f64) -> WalletResult<Self> {
        let max_lamports = sol_to_lamports(max_sol);

        let raw = std::env::var("SOLANA_ARB_TRADING_KEY")
            .map_err(|_| WalletError::FileNotFound(
                "env var 'SOLANA_ARB_TRADING_KEY' not set; \
                 generate with `solana-keygen new` and export the base58 bytes".into(),
            ))?;

        // Decode and immediately wrap in Zeroizing so bytes are wiped on drop.
        let decoded = Zeroizing::new(
            bs58::decode(raw.trim())
                .into_vec()
                .map_err(|e| WalletError::InvalidKeyFormat(format!(
                    "SOLANA_ARB_TRADING_KEY base58 decode: {e}"
                )))?,
        );

        // Reuse the validated crate-internal constructor (bypasses dry_run gate
        // which only applies to the main wallet, not to the trading sub-account).
        let keypair = WalletKeypair::from_64_bytes(&decoded)?;

        info!(
            pubkey = %bs58::encode(keypair.pubkey().as_bytes()).into_string(),
            max_sol,
            "trading sub-account loaded"
        );

        Ok(Self { keypair, main_pubkey, max_lamports })
    }

    /// Return a paper-mode sentinel sub-account (no signing, no RPC calls).
    ///
    /// Suitable for dry-run / test environments — no env var required.
    pub fn paper(main_pubkey: Pubkey, max_sol: f64) -> Self {
        let max_lamports = sol_to_lamports(max_sol);
        Self {
            keypair: WalletKeypair::paper_sentinel(),
            main_pubkey,
            max_lamports,
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// The signing keypair — used to authorise every bot transaction.
    pub fn keypair(&self) -> &WalletKeypair {
        &self.keypair
    }

    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    pub fn main_pubkey(&self) -> Pubkey {
        self.main_pubkey
    }

    pub fn max_lamports(&self) -> u64 {
        self.max_lamports
    }

    pub fn is_paper(&self) -> bool {
        self.keypair.is_paper()
    }

    // ── On-chain balance ──────────────────────────────────────────────────────

    /// Fetch the current on-chain SOL balance in lamports.
    pub async fn balance_lamports(&self, rpc: &RpcClientWrapper) -> WalletResult<u64> {
        rpc.get_balance(&self.pubkey()).await
    }

    /// Returns `true` if the current on-chain balance is within `max_lamports`.
    pub async fn balance_within_cap(&self, rpc: &RpcClientWrapper) -> WalletResult<bool> {
        let bal = self.balance_lamports(rpc).await?;
        Ok(bal <= self.max_lamports)
    }

    // ── Funding ───────────────────────────────────────────────────────────────

    /// Fund the sub-account from `main_keypair` if its balance is below 50% of
    /// `max_lamports`.  Tops it up to exactly `max_lamports`.
    ///
    /// Returns `Some(signature)` if a transfer was made, `None` if sufficient.
    /// No-ops silently in paper mode.
    pub async fn fund_if_needed(
        &self,
        rpc: &RpcClientWrapper,
        main_keypair: &WalletKeypair,
    ) -> WalletResult<Option<String>> {
        if self.is_paper() {
            return Ok(None);
        }

        let current = self.balance_lamports(rpc).await?;
        let threshold = self.max_lamports / 2;

        if current >= threshold {
            info!(
                sub_sol = current as f64 / 1e9,
                "sub-account above 50% threshold, no funding needed"
            );
            return Ok(None);
        }

        let top_up = self.max_lamports.saturating_sub(current);

        // Ensure main wallet can cover the top-up + its own rent + tx fee.
        let main_balance = rpc.get_balance(&main_keypair.pubkey()).await?;
        let required = top_up + RENT_EXEMPT_MIN_LAMPORTS + 5_000;
        if main_balance < required {
            warn!(
                have_sol = main_balance as f64 / 1e9,
                need_sol = required as f64 / 1e9,
                "main wallet insufficient to fund sub-account"
            );
            return Err(WalletError::InsufficientBalance {
                have_sol: main_balance as f64 / 1e9,
                need_sol: required as f64 / 1e9,
            });
        }

        info!(
            top_up_sol = top_up as f64 / 1e9,
            "funding trading sub-account"
        );

        let sig = send_sol_transfer(main_keypair, &self.pubkey(), top_up, rpc).await?;
        Ok(Some(sig))
    }

    // ── Sweep ─────────────────────────────────────────────────────────────────

    /// Sweep all SOL above the rent-exempt minimum back to the main wallet.
    ///
    /// Called on circuit-breaker trip, manual stop, or session end.
    /// Leaves `RENT_EXEMPT_MIN_LAMPORTS` in the sub-account so it stays open.
    ///
    /// Returns `Some(signature)` if swept, `None` if below dust threshold.
    pub async fn sweep_to_main(&self, rpc: &RpcClientWrapper) -> WalletResult<Option<String>> {
        if self.is_paper() {
            return Ok(None);
        }

        let balance = self.balance_lamports(rpc).await?;
        let sweepable = balance.saturating_sub(RENT_EXEMPT_MIN_LAMPORTS);

        if sweepable < MIN_SWEEP_LAMPORTS {
            info!(
                balance_sol = balance as f64 / 1e9,
                "sub-account dust — skipping sweep"
            );
            return Ok(None);
        }

        info!(
            sweepable_sol = sweepable as f64 / 1e9,
            dest = %bs58::encode(self.main_pubkey.as_bytes()).into_string(),
            "sweeping sub-account back to main wallet"
        );

        let sig = send_sol_transfer(&self.keypair, &self.main_pubkey, sweepable, rpc).await?;
        Ok(Some(sig))
    }
}

fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_sub_account_is_paper() {
        let main = Pubkey::new([1u8; 32]);
        let sub = TradingSubAccount::paper(main, 2.0);
        assert!(sub.is_paper());
        assert_eq!(sub.main_pubkey(), main);
        assert_eq!(sub.max_lamports(), 2_000_000_000);
    }

    #[test]
    fn load_fails_when_env_absent() {
        std::env::remove_var("SOLANA_ARB_TRADING_KEY");
        let result = TradingSubAccount::load(Pubkey::new([0u8; 32]), 1.0);
        assert!(matches!(result, Err(WalletError::FileNotFound(_))));
    }

    #[test]
    fn load_fails_on_invalid_base58() {
        std::env::set_var("SOLANA_ARB_TRADING_KEY", "not-valid-base58!!!");
        let result = TradingSubAccount::load(Pubkey::new([0u8; 32]), 1.0);
        std::env::remove_var("SOLANA_ARB_TRADING_KEY");
        assert!(matches!(result, Err(WalletError::InvalidKeyFormat(_))));
    }
}
