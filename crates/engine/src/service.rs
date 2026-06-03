use crate::EngineConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineService {
    pub config: EngineConfig,
}

impl EngineService {
    pub const fn new(config: EngineConfig) -> Self {
        Self { config }
    }
}
