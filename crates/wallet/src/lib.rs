//! `wallet` — the ONLY crate in this workspace that holds live keypair material.
//!
//! All other crates receive only public keys (`Pubkey`), balance snapshots
//! (`BalanceReport`), and signed transactions.  No `Keypair` or secret-key
//! bytes ever leave this crate boundary.
//!
//! # Module layout
//!
//! | Module    | Responsibility |
//! |-----------|---------------|
//! | `keypair` | `WalletKeypair` newtype — loads, holds, and zeroes keypairs |
//! | `rpc`     | Async thin wrapper around `solana_client` nonblocking RPC |
//! | `balance` | `BalanceMonitor` — periodic SOL balance fetching & reporting |
//! | `guard`   | Compile-time and runtime safety guards (paper mode, network, balance) |
//! | `error`   | `WalletError` / `WalletResult` |
//! | `config`  | Re-exports `WalletConfig` from the `config` crate |

#![forbid(unsafe_code)]

pub mod balance;
pub mod config;
pub mod error;
pub mod guard;
pub mod keypair;
pub mod rpc;

// ── Flat re-exports for ergonomic use by integration tests and downstream ──

pub use balance::{BalanceMonitor, BalanceReport};
pub use config::WalletConfig;
pub use error::{WalletError, WalletResult};
pub use guard::{require_live_mode, require_network_match, require_sufficient_balance};
pub use keypair::WalletKeypair;
pub use rpc::{make_rpc, Network, RpcClientWrapper, SimulationResult};
