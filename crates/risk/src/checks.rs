use routing::RouteSearchConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RiskCheckContext {
    pub route_search: RouteSearchConfig,
}

impl Default for RiskCheckContext {
    fn default() -> Self {
        Self {
            route_search: RouteSearchConfig::default(),
        }
    }
}
