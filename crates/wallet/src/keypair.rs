//! `WalletKeypair` — the ONLY place in the workspace where a live keypair exists.
//!
//! # Security guarantees
//!
//! * Does **not** implement `Serialize`, `Clone`, or the auto-derived `Debug`.
//! * `Debug` is implemented manually and never prints secret material.
//! * Secret bytes are zeroed from memory on `Drop` via the `zeroize` crate.
//! * All signing operations panic in paper/dry-run mode.

use config::SystemConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::hash::Hash;
use solana_sdk::transaction::Transaction;
use zeroize::{Zeroize, Zeroizing};

use crate::error::{WalletError, WalletResult};

/// A newtype wrapper around `solana_sdk::signature::Keypair`.
///
/// This struct intentionally does **not** derive `Clone`, `Debug`, or `Serialize`
/// to prevent accidental secret-key leakage.  A custom `Debug` impl exposes
/// only the public key.
pub struct WalletKeypair {
    inner: Keypair,
    /// A copy of the raw 64-byte keypair material, zeroed on drop.
    raw_bytes: Zeroizing<[u8; 64]>,
    is_paper: bool,
}

impl std::fmt::Debug for WalletKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletKeypair")
            .field("pubkey", &self.inner.pubkey().to_string())
            .field("is_paper", &self.is_paper)
            .finish_non_exhaustive()
    }
}

impl Drop for WalletKeypair {
    fn drop(&mut self) {
        // Belt-and-suspenders: Zeroizing already zeroes on its own Drop, but
        // we also call it explicitly so the zeroize happens before the inner
        // Keypair is released from the stack frame.
        (*self.raw_bytes).zeroize();
    }
}

impl WalletKeypair {
    /// Loads a keypair from a Solana CLI JSON file (`[u8; 64]` array format).
    ///
    /// Returns `WalletError::PaperMode` when `cfg.features.dry_run == true`.
    pub fn load_from_file(path: &str, cfg: &SystemConfig) -> WalletResult<Self> {
        if cfg.features.dry_run {
            return Err(WalletError::PaperMode);
        }

        let data = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                WalletError::FileNotFound(path.to_string())
            } else {
                WalletError::Io(e)
            }
        })?;

        let bytes: Vec<u8> = serde_json::from_str(&data)
            .map_err(|e| WalletError::InvalidKeyFormat(e.to_string()))?;

        if bytes.len() != 64 {
            return Err(WalletError::InvalidKeyFormat(format!(
                "expected 64 bytes, got {}",
                bytes.len()
            )));
        }

        let mut raw = [0u8; 64];
        raw.copy_from_slice(&bytes);

        let keypair = Keypair::from_bytes(&raw)
            .map_err(|e| WalletError::InvalidKeyFormat(e.to_string()))?;

        tracing::info!("wallet loaded: pubkey={}", keypair.pubkey());

        Ok(Self {
            inner: keypair,
            raw_bytes: Zeroizing::new(raw),
            is_paper: false,
        })
    }

    /// Loads a keypair from a base58-encoded private key stored in an
    /// environment variable.
    ///
    /// Returns `WalletError::PaperMode` when `cfg.features.dry_run == true`.
    pub fn load_from_env(var: &str, cfg: &SystemConfig) -> WalletResult<Self> {
        if cfg.features.dry_run {
            return Err(WalletError::PaperMode);
        }

        let val = std::env::var(var)
            .map_err(|_| WalletError::FileNotFound(format!("env var '{}' not set", var)))?;

        let decoded = bs58::decode(&val)
            .into_vec()
            .map_err(|e| WalletError::InvalidKeyFormat(e.to_string()))?;

        if decoded.len() != 64 {
            return Err(WalletError::InvalidKeyFormat(format!(
                "expected 64 bytes from base58, got {}",
                decoded.len()
            )));
        }

        let mut raw = [0u8; 64];
        raw.copy_from_slice(&decoded);

        let keypair = Keypair::from_bytes(&raw)
            .map_err(|e| WalletError::InvalidKeyFormat(e.to_string()))?;

        tracing::info!("wallet loaded from env: pubkey={}", keypair.pubkey());

        Ok(Self {
            inner: keypair,
            raw_bytes: Zeroizing::new(raw),
            is_paper: false,
        })
    }

    /// Returns a non-functional sentinel keypair for paper/dry-run mode.
    ///
    /// The key is freshly generated and **never** used for signing.
    /// Any attempt to call [`Self::sign_transaction`] on this sentinel panics.
    pub fn paper_sentinel() -> Self {
        let keypair = Keypair::new();
        let raw = keypair.to_bytes();
        tracing::debug!(
            "paper sentinel created: pubkey={}",
            keypair.pubkey()
        );
        Self {
            inner: keypair,
            raw_bytes: Zeroizing::new(raw),
            is_paper: true,
        }
    }

    /// Returns the public key.  Safe to log/expose.
    pub fn pubkey(&self) -> Pubkey {
        self.inner.pubkey()
    }

    /// Returns `true` if this is a paper-mode sentinel that cannot sign.
    pub fn is_paper(&self) -> bool {
        self.is_paper
    }

    /// Signs a transaction with the loaded keypair.
    ///
    /// # Panics
    ///
    /// Panics immediately if `is_paper == true`.  Call sites must ensure
    /// paper-mode is rejected upstream via [`crate::guard::require_live_mode`].
    pub fn sign_transaction(
        &self,
        tx: &mut Transaction,
        recent_blockhash: Hash,
    ) -> WalletResult<()> {
        if self.is_paper {
            panic!(
                "sign_transaction called on a paper-mode sentinel keypair — \
                 this keypair MUST NOT be used for signing. \
                 Ensure dry_run=false and call require_live_mode() before signing."
            );
        }
        tx.sign(&[&self.inner], recent_blockhash);
        Ok(())
    }
}
