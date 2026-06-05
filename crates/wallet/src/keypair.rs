//! `WalletKeypair` — the ONLY place in the workspace where a live ed25519
//! signing key exists.
//!
//! # Security guarantees
//!
//! * Does **not** implement `Serialize`, `Clone`, or auto-derived `Debug`.
//! * `Debug` is implemented manually and only shows the base58 public key.
//! * Secret bytes are zeroed from memory on `Drop` via the `zeroize` crate.
//! * All signing operations panic in paper/dry-run mode.
//!
//! # Keypair file format
//!
//! Compatible with the Solana CLI JSON keypair file format: a JSON array of
//! 64 `u8` values where bytes `0..32` are the ed25519 secret (seed) and bytes
//! `32..64` are the corresponding public key.

use common::Pubkey;
use config::SystemConfig;
use ed25519_dalek::{SigningKey, Signer};
use zeroize::{Zeroize, Zeroizing};

use crate::error::{WalletError, WalletResult};
use crate::types::Blockhash;

/// A newtype wrapper around an ed25519 signing key.
///
/// Intentionally does NOT derive `Clone`, `Debug`, or `Serialize` to prevent
/// accidental secret-key leakage.  A custom `Debug` impl exposes only the
/// base58-encoded public key.
pub struct WalletKeypair {
    signing_key: SigningKey,
    pubkey: Pubkey,
    /// Raw 64-byte keypair material (secret ‖ public), zeroed on drop.
    raw_bytes: Zeroizing<[u8; 64]>,
    is_paper: bool,
}

impl std::fmt::Debug for WalletKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WalletKeypair")
            .field("pubkey", &bs58::encode(self.pubkey.as_bytes()).into_string())
            .field("is_paper", &self.is_paper)
            .finish_non_exhaustive()
    }
}

impl Drop for WalletKeypair {
    fn drop(&mut self) {
        // Belt-and-suspenders: Zeroizing already zeroes on its own drop, but
        // an explicit call guarantees zeroing happens before any other cleanup.
        (*self.raw_bytes).zeroize();
    }
}

// ── Constructors ──────────────────────────────────────────────────────────────

impl WalletKeypair {
    /// Loads a keypair from a Solana CLI JSON file (64-byte array format).
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

        Self::from_64_bytes(bytes.as_slice()).map(|kp| {
            tracing::info!(
                "wallet loaded: pubkey={}",
                bs58::encode(kp.pubkey.as_bytes()).into_string()
            );
            kp
        })
    }

    /// Loads a keypair from a base58-encoded 64-byte secret stored in an
    /// environment variable.
    ///
    /// Returns `WalletError::PaperMode` when `cfg.features.dry_run == true`.
    pub fn load_from_env(var: &str, cfg: &SystemConfig) -> WalletResult<Self> {
        if cfg.features.dry_run {
            return Err(WalletError::PaperMode);
        }

        let val = std::env::var(var)
            .map_err(|_| WalletError::FileNotFound(format!("env var '{var}' not set")))?;

        let decoded = bs58::decode(&val)
            .into_vec()
            .map_err(|e| WalletError::InvalidKeyFormat(e.to_string()))?;

        Self::from_64_bytes(&decoded).map(|kp| {
            tracing::info!(
                "wallet loaded from env: pubkey={}",
                bs58::encode(kp.pubkey.as_bytes()).into_string()
            );
            kp
        })
    }

    /// Returns a non-functional sentinel keypair for paper/dry-run mode.
    ///
    /// The generated key is ephemeral and **never** used for signing.
    /// Any attempt to call [`Self::sign_message`] on this sentinel panics.
    pub fn paper_sentinel() -> Self {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let pubkey_bytes = signing_key.verifying_key().to_bytes();
        let mut raw = [0u8; 64];
        raw[..32].copy_from_slice(signing_key.as_bytes());
        raw[32..].copy_from_slice(&pubkey_bytes);
        let pubkey = Pubkey::new(pubkey_bytes);

        tracing::debug!(
            "paper sentinel created: pubkey={}",
            bs58::encode(pubkey.as_bytes()).into_string()
        );

        Self {
            signing_key,
            pubkey,
            raw_bytes: Zeroizing::new(raw),
            is_paper: true,
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// Returns the public key.  Safe to log and expose to other crates.
    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    /// Returns `true` if this is a paper-mode sentinel that cannot sign.
    pub fn is_paper(&self) -> bool {
        self.is_paper
    }

    // ── Signing ───────────────────────────────────────────────────────────────

    /// Signs a raw message and returns the 64-byte ed25519 signature.
    ///
    /// # Panics
    ///
    /// Panics immediately if `is_paper == true`.  Call sites must ensure
    /// paper mode is rejected upstream via [`crate::guard::require_live_mode`].
    pub fn sign_message(&self, message: &[u8]) -> WalletResult<[u8; 64]> {
        if self.is_paper {
            panic!(
                "sign_message called on a paper mode sentinel keypair — \
                 this keypair MUST NOT sign. \
                 Ensure dry_run=false and call require_live_mode() before signing."
            );
        }
        Ok(self.signing_key.sign(message).to_bytes())
    }

    /// Convenience alias kept for naming symmetry with Solana transaction flows.
    ///
    /// Signs the serialised transaction bytes (caller must serialise before
    /// calling) and returns the 64-byte signature.
    ///
    /// # Panics
    ///
    /// Panics immediately if `is_paper == true`.
    pub fn sign_transaction_bytes(
        &self,
        tx_bytes: &[u8],
        _blockhash: Blockhash,
    ) -> WalletResult<[u8; 64]> {
        self.sign_message(tx_bytes)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn from_64_bytes(bytes: &[u8]) -> WalletResult<Self> {
        if bytes.len() != 64 {
            return Err(WalletError::InvalidKeyFormat(format!(
                "expected 64 bytes, got {}",
                bytes.len()
            )));
        }

        let secret: [u8; 32] = bytes[..32]
            .try_into()
            .map_err(|_| WalletError::InvalidKeyFormat("slice to array failed".to_string()))?;

        let signing_key = SigningKey::from_bytes(&secret);
        let derived_pub = signing_key.verifying_key().to_bytes();

        // Validate that the stored public key matches the derived one.
        if derived_pub != bytes[32..64] {
            return Err(WalletError::InvalidKeyFormat(
                "public key in file does not match derived key — file may be corrupt".to_string(),
            ));
        }

        let mut raw = [0u8; 64];
        raw.copy_from_slice(bytes);
        let pubkey = Pubkey::new(derived_pub);

        Ok(Self {
            signing_key,
            pubkey,
            raw_bytes: Zeroizing::new(raw),
            is_paper: false,
        })
    }
}
