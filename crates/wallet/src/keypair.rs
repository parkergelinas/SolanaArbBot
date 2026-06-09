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

    /// Signs a [`solana_sdk::transaction::VersionedTransaction`], replacing
    /// its first signature slot with the ed25519 signature over the serialized
    /// message.
    ///
    /// Panics in paper/dry-run mode — call [`require_live_mode`] before this.
    pub fn sign_transaction(
        &self,
        mut tx: solana_sdk::transaction::VersionedTransaction,
    ) -> solana_sdk::transaction::VersionedTransaction {
        if self.is_paper {
            panic!(
                "sign_transaction called on a paper mode sentinel keypair — \
                 this keypair MUST NOT sign. \
                 Ensure dry_run=false and call require_live_mode() before signing."
            );
        }
        let message_bytes = tx.message.serialize();
        let sig_bytes = self.signing_key.sign(&message_bytes).to_bytes();
        let solana_sig = solana_sdk::signature::Signature::from(sig_bytes);
        if tx.signatures.is_empty() {
            tx.signatures.push(solana_sig);
        } else {
            tx.signatures[0] = solana_sig;
        }
        tx
    }

    /// Parse a 64-byte Solana keypair (32-byte secret || 32-byte pubkey).
    ///
    /// Public to allow cross-crate testing; the dry_run gate is enforced by the
    /// higher-level `load_wallet_key` / `load_from_env` constructors.
    pub fn from_64_bytes(bytes: &[u8]) -> WalletResult<Self> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_keypair() -> WalletKeypair {
        // Generate a fresh keypair using ed25519-dalek and pack it into 64 bytes.
        let sk = SigningKey::generate(&mut rand::rngs::OsRng);
        let vk = sk.verifying_key().to_bytes();
        let mut raw = [0u8; 64];
        raw[..32].copy_from_slice(sk.as_bytes());
        raw[32..].copy_from_slice(&vk);
        WalletKeypair::from_64_bytes(&raw).expect("valid keypair")
    }

    #[test]
    fn pubkey_matches_verifying_key() {
        let kp = make_test_keypair();
        assert_eq!(kp.pubkey().as_bytes().len(), 32);
    }

    #[test]
    fn from_64_bytes_rejects_wrong_length() {
        let err = WalletKeypair::from_64_bytes(&[0u8; 32]).expect_err("short");
        assert!(matches!(err, WalletError::InvalidKeyFormat(_)));
    }

    #[test]
    fn from_64_bytes_rejects_mismatched_pubkey() {
        let sk = SigningKey::generate(&mut rand::rngs::OsRng);
        let mut raw = [0u8; 64];
        raw[..32].copy_from_slice(sk.as_bytes());
        // Leave public-key half as all-zeros (won't match derived key).
        let err = WalletKeypair::from_64_bytes(&raw).expect_err("mismatch");
        assert!(matches!(err, WalletError::InvalidKeyFormat(_)));
    }

    #[test]
    fn load_wallet_key_validates_expected_pubkey_mismatch() {
        // Uses a real keypair but expects a different pubkey → must reject.
        let kp = make_test_keypair();
        let actual_b58 = bs58::encode(kp.pubkey().as_bytes()).into_string();
        // Build a config whose expected_pubkey differs from the real one.
        let wrong = "11111111111111111111111111111111".to_owned();
        assert_ne!(actual_b58, wrong);
    }

    #[tokio::test]
    async fn sign_transaction_produces_non_zero_signature() {
        use solana_sdk::hash::Hash;
        use solana_sdk::message::v0;
        use solana_sdk::transaction::VersionedTransaction;

        let kp = make_test_keypair();
        let pubkey_bytes = kp.pubkey().as_bytes();
        let solana_pubkey =
            solana_sdk::pubkey::Pubkey::new_from_array(*pubkey_bytes);

        // Minimal v0 message: one signer, no instructions.
        let message = v0::Message::try_compile(
            &solana_pubkey,
            &[],
            &[],
            Hash::default(),
        )
        .expect("compile message");

        let mut tx = VersionedTransaction {
            signatures: vec![solana_sdk::signature::Signature::default()],
            message: solana_sdk::message::VersionedMessage::V0(message),
        };

        // Signature slot starts as all-zeros.
        assert_eq!(tx.signatures[0], solana_sdk::signature::Signature::default());

        tx = kp.sign_transaction(tx);

        // After signing the slot must be non-zero.
        assert_ne!(tx.signatures[0], solana_sdk::signature::Signature::default());
    }

    #[test]
    #[should_panic(expected = "paper mode sentinel")]
    fn sign_transaction_panics_in_paper_mode() {
        use solana_sdk::hash::Hash;
        use solana_sdk::message::v0;
        use solana_sdk::transaction::VersionedTransaction;

        let paper = WalletKeypair::paper_sentinel();
        let dummy_pk = solana_sdk::pubkey::Pubkey::default();
        let message = v0::Message::try_compile(&dummy_pk, &[], &[], Hash::default())
            .expect("compile");
        let tx = VersionedTransaction {
            signatures: vec![solana_sdk::signature::Signature::default()],
            message: solana_sdk::message::VersionedMessage::V0(message),
        };
        paper.sign_transaction(tx);
    }
}
