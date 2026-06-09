//! `wallet` — the ONLY crate in this workspace that holds live ed25519 signing
//! key material.
//!
//! All other crates receive only public keys ([`common::Pubkey`]), balance
//! snapshots ([`BalanceReport`]), and raw signatures.  No secret key bytes
//! ever leave this crate boundary.
//!
//! # Design notes
//!
//! * Depends on `solana-sdk 2.x` (edition-2021, ed25519-dalek 2.x compatible)
//!   solely for [`solana_sdk::transaction::VersionedTransaction`] signing.
//!   Internal types continue to use [`common::Pubkey`] and [`types::Blockhash`].
//! * All RPC communication uses a minimal `reqwest` JSON-RPC client
//!   instead of `solana-client`.
//!
//! # Module layout
//!
//! | Module    | Responsibility |
//! |-----------|----------------|
//! | `keypair` | `WalletKeypair` — ed25519 keypair loading, holding, and zeroing |
//! | `rpc`     | Async JSON-RPC wrapper for the Solana RPC API |
//! | `balance` | `BalanceMonitor` — periodic SOL balance fetching & reporting |
//! | `guard`   | Runtime safety guards (paper mode, network match, balance) |
//! | `types`   | Wallet-local `Blockhash` primitive |
//! | `error`   | `WalletError` / `WalletResult` |
//! | `config`  | Re-exports `WalletConfig` from the `config` crate |

#![forbid(unsafe_code)]

pub mod balance;
pub mod config;
pub mod error;
pub mod guard;
pub mod keypair;
pub mod rpc;
pub mod sub_account;
pub mod transfer;
pub mod types;

// ── Flat re-exports ───────────────────────────────────────────────────────────

pub use balance::{BalanceMonitor, BalanceReport};
pub use config::WalletConfig;
pub use error::{WalletError, WalletResult};
pub use guard::{
    expected_network, require_live_mode, require_live_mode_for_wallet, require_network_match,
    require_sufficient_balance, validate_network,
};
pub use keypair::WalletKeypair;
pub use rpc::{make_rpc, Network, RpcClientWrapper, SimulationResult};
pub use sub_account::TradingSubAccount;
pub use transfer::{build_signed_transfer_tx, build_transfer_message, send_sol_transfer, RENT_EXEMPT_MIN_LAMPORTS};
pub use types::Blockhash;
