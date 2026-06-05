//! Backward-compatible re-export of the [`storage`] crate.
//!
//! All existing consumers that import from `accounts` continue to work
//! without modification.  New code should import from `storage` directly.

#![forbid(unsafe_code)]

pub use storage::AccountCache;

#[cfg(test)]
mod tests {
    use super::AccountCache;

    #[test]
    fn account_cache_validates_via_re_export() {
        assert!(AccountCache::new().validate().is_ok());
    }
}
