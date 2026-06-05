//! Helius / JSON-RPC adapter stub — future production path.

use crossbeam_channel::Sender;
use tracing::warn;

use super::adapter::IngestionAdapter;
use crate::types::RawUpdate;

pub struct RpcAdapter;

impl IngestionAdapter for RpcAdapter {
    fn name(&self) -> &'static str {
        "rpc"
    }

    fn spawn(self: Box<Self>, _tx: Sender<RawUpdate>) -> anyhow::Result<()> {
        warn!("RpcAdapter (Helius) not implemented — set YELLOWSTONE_ENDPOINT or use mock mode");
        Ok(())
    }
}
