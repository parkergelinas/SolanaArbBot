use accounts::AccountCacheConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PricingContext {
    pub account_cache: AccountCacheConfig,
}

impl Default for PricingContext {
    fn default() -> Self {
        Self {
            account_cache: AccountCacheConfig::default(),
        }
    }
}
