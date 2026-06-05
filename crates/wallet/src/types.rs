//! Minimal Solana primitive types used by the wallet crate.
//!
//! These are lightweight wrappers over raw byte arrays, kept dependency-free
//! so the wallet crate does not pull in the full `solana-sdk` and its heavy
//! transitive dependency tree.

use std::str::FromStr;

use crate::error::{WalletError, WalletResult};

/// A Solana blockhash — 32 raw bytes.
///
/// Displayed and parsed as base58, matching the Solana RPC wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Blockhash(pub [u8; 32]);

impl Blockhash {
    /// Creates a `Blockhash` from raw bytes.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the raw bytes.
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl std::fmt::Display for Blockhash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bs58::encode(&self.0).into_string())
    }
}

impl FromStr for Blockhash {
    type Err = WalletError;

    fn from_str(s: &str) -> WalletResult<Self> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| WalletError::Rpc(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(WalletError::Rpc(format!(
                "invalid blockhash: expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}
