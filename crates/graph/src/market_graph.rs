#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphBuildConfig {
    pub max_edges: usize,
}

impl Default for GraphBuildConfig {
    fn default() -> Self {
        Self { max_edges: 500_000 }
    }
}
