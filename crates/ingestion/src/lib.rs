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
pub mod helius;
pub mod rpc;

pub use engine::{IngestionConfig, IngestionEngine, DEFAULT_PLACEHOLDER_EVENT_LIMIT};
pub use helius::spawn_helius_stream;
pub use rpc::{MockRpcClient, RpcClient, RpcClientInterface};
