//! Wallet configuration — re-exported from the `config` crate.
//!
//! `WalletConfig` is defined in `crates/config/src/schema.rs` to avoid a
//! circular dependency (`wallet` → `config` → `wallet`).  All other crates
//! reference it through either `config::WalletConfig` or this re-export.

pub use config::WalletConfig;
