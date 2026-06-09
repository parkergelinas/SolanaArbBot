pub mod adapter;
pub mod geyser;
pub mod mock;
pub mod pumpswap;
pub mod rpc;

use adapter::IngestionAdapter;
use geyser::GeyserAdapter;
use mock::MockAdapter;

/// Ingestion mode from `DATA_LAYER_INGEST`: `auto` | `mock` | `rpc` | `geyser`.
fn ingest_mode() -> String {
    std::env::var("DATA_LAYER_INGEST")
        .unwrap_or_else(|_| "auto".into())
        .to_lowercase()
}

/// Select adapter: geyser → rpc (live WS) → mock.
pub fn default_adapter() -> Box<dyn IngestionAdapter> {
    match ingest_mode().as_str() {
        "mock" => return Box::new(MockAdapter::from_env()),
        "geyser" => {
            if let Some(g) = GeyserAdapter::from_env() {
                return Box::new(g);
            }
            tracing::warn!("DATA_LAYER_INGEST=geyser but YELLOWSTONE_ENDPOINT unset — falling back");
        }
        "rpc" => {
            if let Some(r) = rpc::RpcAdapter::from_env() {
                return Box::new(r);
            }
            tracing::warn!(
                "DATA_LAYER_INGEST=rpc but no RPC WS URL — set SOLANA_RPC_WS or SOLANA_ARB_RPC__ENDPOINTS"
            );
        }
        _ => {}
    }

    if let Some(geyser) = GeyserAdapter::from_env() {
        return Box::new(geyser);
    }
    if let Some(rpc) = rpc::RpcAdapter::from_env() {
        tracing::info!("data-layer using live RPC log subscription");
        return Box::new(rpc);
    }

    tracing::info!(
        "data-layer using mock ingestion (set SOLANA_ARB_RPC__ENDPOINTS for live chain data)"
    );
    Box::new(MockAdapter::from_env())
}
pub fn spawn_default(tx: crossbeam_channel::Sender<crate::types::RawUpdate>) -> anyhow::Result<()> {
    default_adapter().spawn(tx)
}
