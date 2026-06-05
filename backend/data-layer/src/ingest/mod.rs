pub mod adapter;
pub mod geyser;
pub mod mock;
pub mod rpc;

use adapter::IngestionAdapter;
use geyser::GeyserAdapter;
use mock::MockAdapter;

/// Select adapter: Yellowstone if endpoint set, otherwise mock stream.
pub fn default_adapter() -> Box<dyn IngestionAdapter> {
    if let Some(geyser) = GeyserAdapter::from_env() {
        Box::new(geyser)
    } else {
        Box::new(MockAdapter::from_env())
    }
}

pub fn spawn_default(tx: crossbeam_channel::Sender<crate::types::RawUpdate>) -> anyhow::Result<()> {
    default_adapter().spawn(tx)
}
