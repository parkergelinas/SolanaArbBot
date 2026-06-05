//! Extensible ingestion boundary — mock today, Yellowstone/Helius tomorrow.

use crossbeam_channel::Sender;

use crate::types::RawUpdate;

/// Continuous streaming ingestion adapter (never poll).
pub trait IngestionAdapter: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    /// Spawn background consumer; pushes `RawUpdate` as events arrive.
    fn spawn(self: Box<Self>, tx: Sender<RawUpdate>) -> anyhow::Result<()>;
}
