#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccountCacheConfig {
    pub max_accounts: usize,
}

impl Default for AccountCacheConfig {
    fn default() -> Self {
        Self {
            max_accounts: 1_000_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_cache_config_has_capacity() {
        assert!(AccountCacheConfig::default().max_accounts > 0);
    }
}
