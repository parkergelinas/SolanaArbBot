//! `WalletKeypair` — the ONLY place in the workspace where a live ed25519
//! signing key exists.
//!
//! # Security guarantees
//!
//! * Does **not** implement `Serialize`, `Clone`, or auto-derived `Debug`.
//! * `Debug` is implemented manually and only shows the base58 public key.
//! * Secret bytes are zeroed from memory on drop via the `zeroize` crate.
//! * All signing operations panic in paper/dry-run mode.
//! * Private keys are injected via env var only — never read from disk.

use common::Pubkey;
use config::SystemConfig;
use ed25519_dalek::{SigningKey, Signer};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::error::{WalletError, WalletResult};
use crate::types::Blockhash;

/// Holds raw 64-byte keypair material; zeroed automatically on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
struct KeyMaterial {
    bytes: [u8; 64],
}

/// A newtype wrapper around an ed25519 signing key.
///
/// Intentionally does NOT derive `Clone`, `Debug`, or `Serialize` to prevent
/// accidental secret-key leakage.  A custom `Debug` impl exposes only the
/// base58-encoded public key.
pub struct WalletKeypair {
    material: KeyMaterial,
    signing_key: SigningKey,
    pubkey: Pubkey,
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

// ── Constructors ──────────────────────────────────────────────────────────────

impl WalletKeypair {
    /// Key files are never read from disk — use [`Self::load_from_env`].
    pub fn load_from_file(_path: &str, cfg: &SystemConfig) -> WalletResult<Self> {
        if cfg.features.dry_run {
            return Err(WalletError::PaperMode);
        }
        Err(WalletError::InvalidKeyFormat(
            "key files are not read from disk; inject SOLANA_ARB_WALLET_KEY (base58)".into(),
        ))
    }

    /// Loads a keypair from a base58-encoded 64-byte secret in an env var.
    ///
    /// Prefer `SOLANA_ARB_WALLET_KEY`; falls back to the name in config.
    pub fn load_from_env(var: &str, cfg: &SystemConfig) -> WalletResult<Self> {
        if cfg.features.dry_run {
            return Err(WalletError::PaperMode);
        }

        let val = std::env::var(var)
            .map_err(|_| WalletError::FileNotFound(format!("env var '{var}' not set")))?;

        let decoded = Zeroizing::new(
            bs58::decode(&val)
                .into_vec()
                .map_err(|e| WalletError::InvalidKeyFormat(e.to_string()))?,
        );

        Self::from_64_bytes(&decoded).map(|kp| {
            tracing::info!(
                "wallet loaded from env: pubkey={}",
                bs58::encode(kp.pubkey.as_bytes()).into_string()
            );
            kp
        })
    }

    /// Loads from `SOLANA_ARB_WALLET_KEY` (canonical env var).
    ///
    /// If `wallet.expected_pubkey` is set in config, validates the loaded
    /// keypair's pubkey matches — rejects mismatches to prevent wrong-wallet errors.
    pub fn load_wallet_key(cfg: &SystemConfig) -> WalletResult<Self> {
        let var = cfg
            .wallet
            .keypair_env_var
            .as_deref()
            .unwrap_or("SOLANA_ARB_WALLET_KEY");
        let kp = Self::load_from_env(var, cfg)?;

        if let Some(expected) = &cfg.wallet.expected_pubkey {
            let actual = bs58::encode(kp.pubkey.as_bytes()).into_string();
            if actual != expected.trim() {
                return Err(WalletError::InvalidKeyFormat(format!(
                    "pubkey mismatch: expected {expected}, got {actual}"
                )));
            }
            tracing::info!(pubkey = %actual, "wallet pubkey verified against expected_pubkey");
        }

        Ok(kp)
    }

    /// Returns a non-functional sentinel keypair for paper/dry-run mode.
    pub fn paper_sentinel() -> Self {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let pubkey_bytes = signing_key.verifying_key().to_bytes();
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(signing_key.as_bytes());
        bytes[32..].copy_from_slice(&pubkey_bytes);
        let pubkey = Pubkey::new(pubkey_bytes);

        tracing::debug!(
            "paper sentinel created: pubkey={}",
            bs58::encode(pubkey.as_bytes()).into_string()
        );

        Self {
            material: KeyMaterial { bytes },
            signing_key,
            pubkey,
            is_paper: true,
        }
    }

    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    pub fn is_paper(&self) -> bool {
        self.is_paper
    }

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

    pub fn sign_transaction_bytes(
        &self,
        tx_bytes: &[u8],
        _blockhash: Blockhash,
    ) -> WalletResult<[u8; 64]> {
        self.sign_message(tx_bytes)
    }

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

        if derived_pub != bytes[32..64] {
            return Err(WalletError::InvalidKeyFormat(
                "public key does not match derived key".to_string(),
            ));
        }

        let mut raw = [0u8; 64];
        raw.copy_from_slice(bytes);
        let pubkey = Pubkey::new(derived_pub);

        Ok(Self {
            material: KeyMaterial { bytes: raw },
            signing_key,
            pubkey,
            is_paper: false,
        })
    }
}
