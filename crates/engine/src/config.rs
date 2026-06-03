use accounts::AccountCacheConfig;
use graph::GraphBuildConfig;
use pricing::PricingContext;
use risk::RiskLimits;
use routing::RouteSearchConfig;
use stream::{EventBusConfig, IngestionConfig};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineConfig {
    pub account_cache: AccountCacheConfig,
    pub event_bus: EventBusConfig,
    pub graph: GraphBuildConfig,
    pub ingestion: IngestionConfig,
    pub pricing: PricingContext,
    pub risk_limits: RiskLimits,
    pub route_search: RouteSearchConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            account_cache: AccountCacheConfig::default(),
            event_bus: EventBusConfig::default(),
            graph: GraphBuildConfig::default(),
            ingestion: IngestionConfig::default(),
            pricing: PricingContext::default(),
            risk_limits: RiskLimits::default(),
            route_search: RouteSearchConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_config_composes_subsystem_defaults() {
        let config = EngineConfig::default();

        assert!(config.account_cache.max_accounts > 0);
        assert!(config.route_search.max_hops > 0);
    }
}
