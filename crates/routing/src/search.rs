#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteSearchConfig {
    pub max_hops: usize,
    pub max_candidates: usize,
}

impl Default for RouteSearchConfig {
    fn default() -> Self {
        Self {
            max_hops: 4,
            max_candidates: 128,
        }
    }
}
