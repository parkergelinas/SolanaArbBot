#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccountLoaderConfig {
    pub batch_size: usize,
}

impl Default for AccountLoaderConfig {
    fn default() -> Self {
        Self { batch_size: 256 }
    }
}
