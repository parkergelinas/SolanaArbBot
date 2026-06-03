//! Account state boundary.
//!
//! This crate will own normalized Solana account views used by analysis stages.

#![forbid(unsafe_code)]

pub mod cache {
    //! Account cache ownership and lookup responsibilities.

    use common::Result;

    /// Placeholder account cache handle.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct AccountCache;

    impl AccountCache {
        /// Creates an empty placeholder cache.
        #[must_use]
        pub const fn new() -> Self {
            Self
        }

        /// Performs a no-op validation for the scaffold.
        pub const fn validate(&self) -> Result<()> {
            Ok(())
        }
    }
}

pub use cache::AccountCache;

#[cfg(test)]
mod tests {
    use super::AccountCache;

    #[test]
    fn account_cache_placeholder_validates() {
        assert!(AccountCache::new().validate().is_ok());
    }
}
