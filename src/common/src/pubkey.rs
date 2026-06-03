//! Lightweight public key wrapper used across the workspace.

use core::fmt;

use crate::{Error, Result};

/// Number of bytes in a Solana public key.
pub const PUBKEY_BYTES: usize = 32;

/// Solana public key representation.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    /// Creates a public key wrapper from raw bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Creates a public key wrapper from a byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> Result<Self> {
        Self::try_from(bytes)
    }

    /// Returns the raw 32-byte key.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the raw key bytes by value.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Pubkey").field(&self.0).finish()
    }
}

impl From<[u8; PUBKEY_BYTES]> for Pubkey {
    fn from(bytes: [u8; PUBKEY_BYTES]) -> Self {
        Self::new(bytes)
    }
}

impl From<Pubkey> for [u8; PUBKEY_BYTES] {
    fn from(pubkey: Pubkey) -> Self {
        pubkey.to_bytes()
    }
}

impl TryFrom<&[u8]> for Pubkey {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != PUBKEY_BYTES {
            return Err(Error::InvalidState(format!(
                "invalid pubkey length: expected {PUBKEY_BYTES}, got {}",
                bytes.len()
            )));
        }

        let mut pubkey = [0; PUBKEY_BYTES];
        pubkey.copy_from_slice(bytes);
        Ok(Self::new(pubkey))
    }
}

#[cfg(test)]
mod tests {
    use super::{Pubkey, PUBKEY_BYTES};
    use crate::Error;

    #[test]
    fn pubkey_round_trips_bytes() {
        let bytes = [9; PUBKEY_BYTES];
        let pubkey = Pubkey::new(bytes);

        assert_eq!(pubkey.as_bytes(), &bytes);
        assert_eq!(pubkey.to_bytes(), bytes);
        assert_eq!(<[u8; PUBKEY_BYTES]>::from(pubkey), bytes);
    }

    #[test]
    fn pubkey_rejects_invalid_slice_length() {
        let err = Pubkey::try_from_slice(&[1, 2, 3]).expect_err("invalid length");

        assert!(matches!(err, Error::InvalidState(_)));
    }
}
