//! Account cache ownership and lookup responsibilities.

use common::Result;

/// In-memory account state cache.
///
/// Currently a placeholder; will hold normalized on-chain account data
/// (pool state, token mints, etc.) as the system matures.
#[derive(Clone, Copy, Debug, Default)]
pub struct AccountCache;

impl AccountCache {
    /// Creates an empty account cache.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Validates cache invariants.
    pub const fn validate(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AccountCache;

    #[test]
    fn account_cache_placeholder_validates() {
        assert!(AccountCache::new().validate().is_ok());
    }
}
