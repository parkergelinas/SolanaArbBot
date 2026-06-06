//! Data ingestion boundary.
//!
//! Canonical home for:
//! - [`rpc`]: transport-agnostic [`RpcClient`] trait and [`MockRpcClient`]
//! - [`engine`]: async [`IngestionEngine`] that publishes to the event bus
//!
//! The legacy [`stream`] and [`rpc_client`] workspace crates are thin re-export
//! wrappers over this crate for backward compatibility.

#![forbid(unsafe_code)]

pub mod engine;
pub mod geyser;
pub mod helius;
pub mod rpc;
pub mod sniper_ingest;

pub use engine::{IngestionConfig, IngestionEngine, DEFAULT_PLACEHOLDER_EVENT_LIMIT};
pub use helius::spawn_helius_stream;
pub use rpc::{MockRpcClient, RpcClient, RpcClientInterface};
pub use sniper_ingest::{
    parse_pool_creation_from_logs, parse_pool_creation_notification, parse_pump_trade_from_logs,
    parse_pump_trade_notification, spawn_pump_trade_ingest, spawn_sniper_ingest, PoolCreationEvent,
    PoolCreationSource, PumpTradeEvent, PumpTradeSide, PUMP_FUN, PUMP_SWAP, RAYDIUM_AMM_V4,
    RAYDIUM_CLMM, SOL_MINT,
};
